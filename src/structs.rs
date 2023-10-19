#[derive(Debug, serde::Deserialize)]
pub struct Record {
    pub name: String,
    pub url: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct RecordWithIp {
    pub name: String,
    pub url: String,
    pub ip: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct RecordWithGeolocation {
    pub ip: String,
    pub location: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct RecordWithTime {
    pub ip: String,
    pub location: String,
    pub time: f64,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct RecordWithDistance {
    pub ip: String,
    pub location: String,
    pub time: f64,
    pub distance: f64,
}
