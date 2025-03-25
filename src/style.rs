//! Style information for YAML nodes

/// Style for sequence nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TSequenceStyle {
    /// Flow style sequence [1, 2, 3]
    Flow,
    /// Block style sequence with dash indicators
    /// - 1
    /// - 2
    Block,
}

/// Style for mapping nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TMappingStyle {
    /// Flow style mapping {key: value}
    Flow,
    /// Block style mapping
    /// key: value
    Block,
}

/// Combined style information for different node types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeStyle {
    /// Style for sequence nodes
    Sequence(TSequenceStyle),
    /// Style for mapping nodes
    Mapping(TMappingStyle),
}

impl NodeStyle {
    /// Returns true if this style is a flow style (either sequence or mapping)
    pub fn is_flow(&self) -> bool {
        match self {
            NodeStyle::Sequence(TSequenceStyle::Flow) => true,
            NodeStyle::Mapping(TMappingStyle::Flow) => true,
            _ => false,
        }
    }

    /// Returns true if this style is a block style (either sequence or mapping)
    pub fn is_block(&self) -> bool {
        !self.is_flow()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_sequence_style_equality() {
        assert_eq!(TSequenceStyle::Flow, TSequenceStyle::Flow);
        assert_eq!(TSequenceStyle::Block, TSequenceStyle::Block);
        assert_ne!(TSequenceStyle::Flow, TSequenceStyle::Block);
    }

    #[test]
    fn test_mapping_style_equality() {
        assert_eq!(TMappingStyle::Flow, TMappingStyle::Flow);
        assert_eq!(TMappingStyle::Block, TMappingStyle::Block);
        assert_ne!(TMappingStyle::Flow, TMappingStyle::Block);
    }

    #[test]
    fn test_node_style_flow_detection() {
        let flow_seq = NodeStyle::Sequence(TSequenceStyle::Flow);
        let block_seq = NodeStyle::Sequence(TSequenceStyle::Block);
        let flow_map = NodeStyle::Mapping(TMappingStyle::Flow);
        let block_map = NodeStyle::Mapping(TMappingStyle::Block);

        assert!(flow_seq.is_flow());
        assert!(!block_seq.is_flow());
        assert!(flow_map.is_flow());
        assert!(!block_map.is_flow());

        assert!(!flow_seq.is_block());
        assert!(block_seq.is_block());
        assert!(!flow_map.is_block());
        assert!(block_map.is_block());
    }
}
