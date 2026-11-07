use gpui::Hsla;

#[derive(Clone, Copy, Debug)]
pub struct DataPoint {
    pub time: f64,
    pub value: f64,
    pub index: usize,
}

#[derive(Clone, Debug)]
pub struct Series {
    pub name: String,
    pub unit: Option<String>,
    /// Decimal places for displaying this signal's values.
    pub decimals: usize,
    pub points: Vec<DataPoint>,
    pub color: Hsla,
    pub time_labels: Vec<String>, // Pre-formatted time strings for x-axis
}
