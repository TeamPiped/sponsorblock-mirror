use std::collections::HashMap;

use diesel::prelude::*;
use lazy_static::lazy_static;
use rocket::response::content;

use crate::{Db, Segment, Sponsor};
use crate::models::SponsorTime;
use crate::schema::sponsorTimes::dsl::*;

// init regex to match hash/hex
lazy_static! {
    static ref RE: regex::Regex = regex::Regex::new(r"^[0-9a-f]{4}$").unwrap();
}

#[get("/api/skipSegments/<hash>?<categories>")]
pub async fn skip_segments(
    hash: String,
    categories: Option<&str>,
    db: Db,
) -> content::RawJson<String> {
    let hash = hash.to_lowercase();

    // Check if hash matches hex regex
    if !RE.is_match(&hash) {
        return content::RawJson("Hash prefix does not match format requirements.".to_string());
    }

    let hc = hash.clone();

    let cat: Vec<String> = serde_json::from_str(categories.unwrap_or("[]")).unwrap();

    if cat.is_empty() && categories.is_some() {
        return content::RawJson(
            "[]".to_string(),
        );
    }

    let results: Vec<SponsorTime> = db.run(move |conn| {
        let base_filter = sponsorTimes
            .filter(shadowHidden.eq(0))
            .filter(hidden.eq(0))
            .filter(votes.ge(0))
            .filter(hashedVideoID.like(format!("{}%", hc)));

        let queried = {
            if cat.is_empty() {
                base_filter
                    .get_results::<SponsorTime>(conn)
                    .expect("Failed to query sponsor times")
            } else {
                base_filter
                    .filter(category.eq_any(cat))
                    .get_results::<SponsorTime>(conn)
                    .expect("Failed to query sponsor times")
            }
        };

        queried
    }).await;

    // Create map of Sponsors - Hash, Sponsor
    let mut sponsors: HashMap<String, Sponsor> = HashMap::new();

    for result in results {
        let sponsor = {
            sponsors.entry(result.hashed_video_id.clone()).or_insert(Sponsor {
                hash: result.hashed_video_id,
                video_id: result.video_id,
                segments: Vec::new(),
            })
        };

        sponsor.segments.push(Segment {
            uuid: result.uuid,
            action_type: result.action_type,
            category: result.category,
            description: result.description,
            locked: result.locked,
            segment: vec![result.start_time, result.end_time],
            user_id: result.user_id,
            video_duration: result.video_duration,
            votes: result.votes,
        });
    }

    if !sponsors.is_empty() {
        let sponsors: Vec<&Sponsor> = sponsors.values().collect();
        return content::RawJson(serde_json::to_string(&sponsors).unwrap());
    }

    let resp = reqwest::get(format!(
        "https://sponsor.ajay.app/api/skipSegments/{}?categories={}",
        hash,
        categories.unwrap_or("[]"),
    ))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    return content::RawJson(resp);
}
