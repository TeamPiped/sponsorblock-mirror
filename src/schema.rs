// @generated automatically by Diesel CLI.

diesel::table! {
    #[allow(non_snake_case)]
    sponsorTimes (UUID) {
        videoID -> Text,
        startTime -> Float4,
        endTime -> Float4,
        votes -> Int4,
        locked -> Int4,
        incorrectVotes -> Int4,
        UUID -> Text,
        userID -> Text,
        timeSubmitted -> Int8,
        views -> Int4,
        category -> Text,
        actionType -> Text,
        service -> Text,
        videoDuration -> Float4,
        hidden -> Int4,
        reputation -> Float4,
        shadowHidden -> Int4,
        hashedVideoID -> Text,
        userAgent -> Text,
        description -> Text,
    }
}
