use std::collections::HashMap;

use diesel::prelude::*;
use lazy_static::lazy_static;
use rocket::response::content;

use crate::{Db, Segment, Sponsor};
use crate::models::SponsorTime;
// We *must* use "videoID" as an argument name to get Rocket to let us access
// the query parameter by that name, but if videoID is already used we
// can't do that.
use crate::schema::sponsorTimes::dsl::{
    sponsorTimes,
    shadowHidden,
    hidden,
    votes,
    category,
    hashedVideoID,
    videoID as column_videoID
};

// init regexes to match hash/hex or video ID
lazy_static! {
    static ref HASH_RE: regex::Regex = regex::Regex::new(r"^[0-9a-f]{4}$").unwrap();
    static ref ID_RE: regex::Regex = regex::Regex::new(r"^[a-zA-Z0-9_-]{6,11}$").unwrap();
}

// Segments can be fetched either by full video ID, or by prefix of hashed
// video ID. Different clients make different queries. This represents either
// kind of constraint.
enum VideoName {
    ByHashPrefix(String),
    ByID(String),
}


#[get("/api/skipSegments/<hash>?<categories>")]
pub async fn skip_segments(
    hash: String,
    categories: Option<&str>,
    db: Db,
) -> content::RawJson<String> {

    let hash = hash.to_lowercase();

    // Check if hash matches hex regex
    if !HASH_RE.is_match(&hash) {
        return content::RawJson("Hash prefix does not match format requirements.".to_string());
    }

    let search_result = find_skip_segments(VideoName::ByHashPrefix(hash.clone()), categories, db).await;
    
    match search_result {
        Some(segments) => return segments,
        None => {
            // Fall back to central Sponsorblock server
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
    }
}

#[get("/api/skipSegments?<videoID>&<categories>")]
pub async fn skip_segments_by_id(
    #[allow(non_snake_case)]
    videoID: String,
    categories: Option<&str>,
    db: Db,
) -> content::RawJson<String> {

    if videoID.is_empty()  {
        return content::RawJson("videoID is missing".to_string());
    }

    let search_result = find_skip_segments(VideoName::ByID(videoID.clone()), categories, db).await;
    
    match search_result {
        Some(segments) => return segments,
        None => {
            // Fall back to central Sponsorblock server
            let resp = reqwest::get(format!(
                "https://sponsor.ajay.app/api/skipSegments?videoID={}&categories={}",
                videoID,
                categories.unwrap_or("[]"),
            ))
                .await
                .unwrap()
                .text()
                .await
                .unwrap();

            return content::RawJson(resp);
        }
    }
}

async fn find_skip_segments(
    name: VideoName,
    categories: Option<&str>,
    db: Db,
) -> Option<content::RawJson<String>> {

    let cat: Vec<String> = serde_json::from_str(categories.unwrap_or("[\"sponsor\"]")).unwrap();

    if cat.is_empty() {
        return Some(content::RawJson(
            "[]".to_string(),
        ));
    }
    
    let results: Vec<SponsorTime> = db.run(move |conn| {
        let base_filter = sponsorTimes
            .filter(shadowHidden.eq(0))
            .filter(hidden.eq(0))
            .filter(votes.ge(0))
            .filter(category.eq_any(cat)); // We know cat isn't empty at this point
        
        let queried = match name {
            VideoName::ByHashPrefix(hash_prefix) => {
                base_filter
                    .filter(hashedVideoID.like(format!("{}%", hash_prefix)))
                    .get_results::<SponsorTime>(conn)
                    .expect("Failed to query sponsor times")
            }
            VideoName::ByID(video_id) => {
                base_filter
                    .filter(column_videoID.eq(video_id))
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

    for sponsor in sponsors.values_mut() {
        sponsor.segments.sort_by(|a, b| a.partial_cmp(b).unwrap());
    }

    if !sponsors.is_empty() {
        let sponsors: Vec<&Sponsor> = sponsors.values().collect();
        return Some(content::RawJson(serde_json::to_string(&sponsors).unwrap()));
    }

    return None;
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

// We might need some more routes to support ReVanced. These are faked for now.
// Sadly not even these are sufficient to make it work for me, maybe it is just
// broken?

// This would take a userID
#[get("/api/isUserVIP")]
pub async fn fake_is_user_vip() -> content::RawJson<String> {
    content::RawJson("{\"hashedUserID\": \"\", \"vip\": false}".to_string())
}

// This would take a userID and an optional list values
#[get("/api/userInfo")]
pub async fn fake_user_info() -> content::RawJson<String> {
    content::RawJson("{\"userID\": \"\", \"userName\": \"\", \"minutesSaved\": 0, \"segmentCount\": 0, \"viewCount\": 0}".to_string())
}
