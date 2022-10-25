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

    let cat: Vec<String> = serde_json::from_str(categories.unwrap_or("[\"sponsor\"]")).unwrap();

    if cat.is_empty() {
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

    for result in &results {
        let sponsor = {
            sponsors.entry(result.hashed_video_id.clone()).or_insert(Sponsor {
                hash: result.hashed_video_id.clone(),
                video_id: result.video_id.clone(),
                segments: Vec::new(),
            })
        };

        let segment = Segment {
            uuid: result.uuid.clone(),
            action_type: result.action_type.clone(),
            category: result.category.clone(),
            description: result.description.clone(),
            locked: result.locked,
            segment: vec![result.start_time, result.end_time],
            user_id: result.user_id.clone(),
            video_duration: result.video_duration,
            votes: result.votes,
        };

        let hash = result.hashed_video_id.clone();

        let mut found_similar = false;

        for seg in &sponsor.segments {
            if is_overlap(&segment, &seg.category, &seg.action_type, seg.segment[0], seg.segment[1]) {
                found_similar = true;
                break;
            }
        }

        if found_similar {
            continue;
        }

        let mut similar_segments = similar_segments(&segment, &hash, &results);
        similar_segments.push(segment.clone());

        let best_segment = best_segment(&similar_segments);

        // Add if not already in sponsor
        if !sponsor.segments.contains(&best_segment) {
            sponsor.segments.push(best_segment);
        }
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

fn similar_segments(segment: &Segment, hash: &str, segments: &Vec<SponsorTime>) -> Vec<Segment> {
    let mut similar_segments: Vec<Segment> = Vec::new();

    for seg in segments {
        if seg.uuid == segment.uuid {
            continue;
        }

        if seg.hashed_video_id != hash {
            continue;
        }

        let is_similar = is_overlap(segment, &seg.category, &seg.action_type, seg.start_time, seg.end_time);

        if is_similar {
            similar_segments.push(Segment {
                uuid: seg.uuid.clone(),
                action_type: seg.action_type.clone(),
                category: seg.category.clone(),
                description: seg.description.clone(),
                locked: seg.locked,
                segment: vec![seg.start_time, seg.end_time],
                user_id: seg.user_id.clone(),
                video_duration: seg.video_duration,
                votes: seg.votes,
            });
        }
    }

    similar_segments
}

fn is_overlap(seg: &Segment, cat: &str, action_type: &str, start: f32, end: f32) -> bool {
    if seg.category != cat {
        return false;
    }

    if seg.segment[0] > start && seg.segment[1] < end {
        return true;
    }
    let overlap = f32::min(seg.segment[1], end) - f32::max(seg.segment[0], start);
    let duration = f32::max(seg.segment[1], end) - f32::min(seg.segment[0], start);
    overlap / duration > {
        if cat == "chapter" {
            0.8
        } else if seg.action_type == action_type {
            0.6
        } else {
            0.1
        }
    }
}

fn best_segment(segments: &Vec<Segment>) -> Segment {
    let mut best_segment = segments[0].clone();
    let mut best_votes = segments[0].votes;

    for segment in segments {
        if segment.votes > best_votes {
            best_segment = segment.clone();
            best_votes = segment.votes;
        }
    }

    best_segment
}