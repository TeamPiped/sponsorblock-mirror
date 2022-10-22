CREATE UNLOGGED TABLE "sponsorTimes"
(
    "videoID"        TEXT    NOT NULL,
    "startTime"      REAL    NOT NULL,
    "endTime"        REAL    NOT NULL,
    "votes"          INTEGER NOT NULL,
    "locked"         INTEGER NOT NULL default '0',
    "incorrectVotes" INTEGER NOT NULL default '1',
    "UUID"           TEXT    NOT NULL UNIQUE PRIMARY KEY,
    "userID"         TEXT    NOT NULL,
    "timeSubmitted"  BIGINT NOT NULL,
    "views"          INTEGER NOT NULL,
    "category"       TEXT    NOT NULL DEFAULT 'sponsor',
    "actionType"     TEXT    NOT NULL DEFAULT 'skip',
    "service"        TEXT    NOT NULL DEFAULT 'YouTube',
    "videoDuration"  REAL    NOT NULL DEFAULT '0',
    "hidden"         INTEGER NOT NULL DEFAULT '0',
    "reputation"     REAL    NOT NULL DEFAULT 0,
    "shadowHidden"   INTEGER NOT NULL,
    "hashedVideoID"  TEXT    NOT NULL default '',
    "userAgent"      TEXT    NOT NULL default '',
    "description"    TEXT    NOT NULL default ''
);
