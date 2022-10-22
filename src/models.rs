use diesel::prelude::*;
use serde::Serialize;
use crate::schema::*;

#[derive(Debug, Serialize, Queryable, Insertable)]
#[diesel(table_name = sponsorTimes)]
pub struct SponsorTime {
    #[serde(rename = "videoID")]
    #[diesel(column_name = videoID)]
    pub video_id: String,
    #[serde(rename = "startTime")]
    #[diesel(column_name = startTime)]
    pub start_time: f32,
    #[serde(rename = "endTime")]
    #[diesel(column_name = endTime)]
    pub end_time: f32,
    pub votes: i32,
    pub locked: i32,
    #[serde(rename = "incorrectVotes")]
    #[diesel(column_name = incorrectVotes)]
    pub incorrect_votes: i32,
    #[serde(rename = "UUID")]
    #[diesel(column_name = UUID)]
    pub uuid: String,
    #[serde(rename = "userID")]
    #[diesel(column_name = userID)]
    pub user_id: String,
    #[serde(rename = "timeSubmitted")]
    #[diesel(column_name = timeSubmitted)]
    pub time_submitted: i64,
    pub views: i32,
    pub category: String,
    #[serde(rename = "actionType")]
    #[diesel(column_name = actionType)]
    pub action_type: String,
    pub service: String,
    #[serde(rename = "videoDuration")]
    #[diesel(column_name = videoDuration)]
    pub video_duration: f32,
    pub hidden: i32,
    pub reputation: f32,
    #[serde(rename = "shadowHidden")]
    #[diesel(column_name = shadowHidden)]
    pub shadow_hidden: i32,
    #[serde(rename = "hashedVideoID")]
    #[diesel(column_name = hashedVideoID)]
    pub hashed_video_id: String,
    #[serde(rename = "userAgent")]
    #[diesel(column_name = userAgent)]
    pub user_agent: String,
    pub description: String,
}
