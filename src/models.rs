use serde::Deserialize;

#[derive(Debug, Clone, PartialEq)]
pub enum QuoteLength {
    Short,
    Medium,
    Long,
    VeryLong,
    All,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QuoteSelector {
    Category(QuoteLength),
    Id(usize),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Time(u64),
    Words(usize),
    Quote(QuoteSelector),
}

#[derive(Debug, PartialEq)]
pub enum AppState {
    Waiting,
    Running,
    Finished,
}

#[derive(Debug, Deserialize, Clone)]
pub struct QuoteEntry {
    pub text: String,
    pub source: String,
    pub length: usize,
    pub id: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct QuoteData {
    #[allow(dead_code)]
    pub language: String,
    pub groups: Vec<Vec<usize>>,
    pub quotes: Vec<QuoteEntry>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WordData {
    #[allow(dead_code)]
    pub name: String,
    pub words: Vec<String>,
}
