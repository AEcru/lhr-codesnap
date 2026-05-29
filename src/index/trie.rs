use ahash::HashMap;

/// Radix-style prefix trie for fast symbol name completion and prefix search.
///
/// Each node stores children keyed by the first character of remaining suffix.
/// Leaf nodes hold the associated name_id.
pub struct SymbolTrie {
    root: TrieNode,
    /// Direct lookup cache for exact matches (O(1) bypass)
    exact_cache: HashMap<String, u32>,
}

#[derive(Default)]
struct TrieNode {
    children: HashMap<char, TrieNode>,
    name_id: Option<u32>,
    /// Compressed edge label (remainder of the string from this node)
    label: String,
}

impl SymbolTrie {
    pub fn new() -> Self {
        Self { root: TrieNode::default(), exact_cache: HashMap::default() }
    }

    /// Insert a symbol name with its name_id.
    pub fn insert(&mut self, name: &str, name_id: u32) {
        self.exact_cache.insert(name.to_string(), name_id);
        Self::insert_recursive(&mut self.root, name, name_id);
    }

    fn insert_recursive(node: &mut TrieNode, remaining: &str, name_id: u32) {
        if remaining.is_empty() {
            node.name_id = Some(name_id);
            return;
        }

        let first = remaining.chars().next().unwrap();
        let child = node.children.entry(first).or_insert_with(|| {
            TrieNode { label: remaining.to_string(), ..Default::default() }
        });

        // Find common prefix with existing label
        let common_len = common_prefix_len(&child.label, remaining);
        if common_len < child.label.len() {
            // Split the existing node
            let old_suffix = child.label[common_len..].to_string();
            let old_name_id = child.name_id.take();
            let old_children = std::mem::take(&mut child.children);

            let mut split_node = TrieNode::default();
            split_node.label = old_suffix.clone();
            split_node.name_id = old_name_id;
            split_node.children = old_children;

            child.label = remaining[..common_len].to_string();
            if common_len == remaining.len() {
                child.name_id = Some(name_id);
            }
            let split_key = old_suffix.chars().next().unwrap();
            child.children.insert(split_key, split_node);

            if common_len < remaining.len() {
                let new_suffix = &remaining[common_len..];
                let new_key = new_suffix.chars().next().unwrap();
                let mut new_node = TrieNode::default();
                new_node.label = new_suffix.to_string();
                new_node.name_id = Some(name_id);
                child.children.insert(new_key, new_node);
            }
        } else if common_len == remaining.len() {
            child.name_id = Some(name_id);
        } else {
            // remaining is longer, recurse
            let new_remaining = &remaining[common_len..];
            Self::insert_recursive(child, new_remaining, name_id);
        }
    }

    /// Exact lookup by symbol name. O(1) via cache.
    pub fn exact_lookup(&self, name: &str) -> Option<u32> {
        self.exact_cache.get(name).copied()
    }

    /// Prefix search: find all name_ids whose name starts with `prefix`.
    /// Reserved for fuzzy matching optimization (tree walk vs linear scan).
    #[allow(dead_code)]
    pub fn prefix_search(&self, prefix: &str) -> Vec<u32> {
        let mut results = Vec::new();
        Self::collect_prefix(&self.root, prefix, prefix, &mut results);
        results
    }

    #[allow(dead_code)]
    fn collect_prefix(node: &TrieNode, original_prefix: &str, remaining: &str, results: &mut Vec<u32>) {
        if remaining.is_empty() {
            // Prefix exhausted, collect all descendants
            if let Some(id) = node.name_id {
                results.push(id);
            }
            for child in node.children.values() {
                Self::collect_all(child, results);
            }
            return;
        }

        if node.label == original_prefix {
            // At root, find the right starting child
            let first = remaining.chars().next().unwrap();
            if let Some(child) = node.children.get(&first) {
                if remaining.starts_with(&child.label) {
                    let new_remaining = &remaining[child.label.len()..];
                    if new_remaining.is_empty() {
                        if let Some(id) = child.name_id {
                            results.push(id);
                        }
                        for grandchild in child.children.values() {
                            Self::collect_all(grandchild, results);
                        }
                    } else {
                        Self::collect_prefix(child, original_prefix, new_remaining, results);
                    }
                } else if child.label.starts_with(remaining) {
                    // Prefix is a prefix of the child label
                    if let Some(id) = child.name_id {
                        results.push(id);
                    }
                    for grandchild in child.children.values() {
                        Self::collect_all(grandchild, results);
                    }
                }
            }
        }
    }

    #[allow(dead_code)]
    fn collect_all(node: &TrieNode, results: &mut Vec<u32>) {
        if let Some(id) = node.name_id {
            results.push(id);
        }
        for child in node.children.values() {
            Self::collect_all(child, results);
        }
    }
}

fn common_prefix_len(a: &str, b: &str) -> usize {
    a.chars().zip(b.chars()).take_while(|(ca, cb)| ca == cb).count()
}
