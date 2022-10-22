use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Sponsor {
    pub hash: String,
    #[serde(rename = "video_id")]
    pub video_id: String,
    pub segments: Vec<Segment>,
}


#[derive(Serialize, Deserialize)]
pub struct Segment {
    #[serde(rename = "uuid")]
    pub uuid: String,
    #[serde(rename = "action_type")]
    pub action_type: String,
    pub category: String,
    pub description: String,
    pub locked: i32,
    pub segment: Vec<f32>,
    #[serde(rename = "user_id")]
    pub user_id: String,
    #[serde(rename = "video_duration")]
    pub video_duration: f32,
    pub votes: i32,
}
