use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ItemType {
    Text,
    Code,
    Image,
    File,
    Files,
    Link,
    Character,
    Color,
}

impl ToString for ItemType {
    fn to_string(&self) -> String {
        match self {
            Self::Text => "Text".to_string(),
            Self::Code => "Code".to_string(),
            Self::Image => "Image".to_string(),
            Self::File => "File".to_string(),
            Self::Files => "Files".to_string(),
            Self::Link => "Link".to_string(),
            Self::Character => "Character".to_string(),
            Self::Color => "Color".to_string(),
        }
    }
}

impl std::str::FromStr for ItemType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Text" => Ok(Self::Text),
            "Code" => Ok(Self::Code),
            "Image" => Ok(Self::Image),
            "File" => Ok(Self::File),
            "Files" => Ok(Self::Files),
            "Link" => Ok(Self::Link),
            "Character" => Ok(Self::Character),
            "Color" => Ok(Self::Color),
            _ => Err(format!("Unknown item type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardEntry {
    pub id: Option<i64>,
    pub item_type: ItemType,
    pub content: String,
    pub pinned: bool,
    pub tag: Option<String>,
    pub datetime: String, // formato: YYYY-MM-DD HH:MM:SS
    pub metadata: Option<String>,
    pub title: Option<String>,
}
