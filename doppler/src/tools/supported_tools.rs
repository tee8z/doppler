use crate::{CloneableHashMap, Rule};
use pest::iterators::Pair;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub enum SupportedTool {
    #[default]
    Esplora,
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct ToolImageInfo {
    tag: String,
    name: String,
    is_custom: bool,
    tool_type: SupportedTool,
}

impl ToolImageInfo {
    pub fn new(
        tag: String,
        name: String,
        is_custom: bool,
        tool_type: SupportedTool,
    ) -> ToolImageInfo {
        ToolImageInfo {
            tag,
            name,
            is_custom,
            tool_type,
        }
    }
    pub fn get_image(&self) -> String {
        if self.is_custom {
            self.tag.clone()
        } else {
            match self.tool_type {
                SupportedTool::Esplora => String::from("blockstream/esplora:latest"),
            }
        }
    }
    pub fn is_image(&self, name: &str) -> bool {
        self.name == name
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }
    pub fn get_tag(&self) -> String {
        self.tag.clone()
    }
}

impl TryFrom<Pair<'_, Rule>> for SupportedTool {
    type Error = String;

    fn try_from(pair: Pair<Rule>) -> Result<Self, Self::Error> {
        match pair.as_str() {
            "ESPLORA" => Ok(SupportedTool::Esplora),
            _ => Err(format!("Unknown tool type: {}", pair.as_str())),
        }
    }
}

pub fn get_supported_tool_images() -> CloneableHashMap<SupportedTool, ToolImageInfo> {
    let mut hash_map = CloneableHashMap::new();
    // NOTE: safe to use * as name since the grammar of the parse wont allow for special characters for the image name, only for the image tag
    hash_map.insert(
        SupportedTool::Esplora,
        ToolImageInfo::new(
            String::from("blockstream/esplora:latest"),
            String::from("*5"),
            false,
            SupportedTool::Esplora,
        ),
    );
    hash_map
}
