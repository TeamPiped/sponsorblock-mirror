CREATE INDEX sponsor_category_idx ON "sponsorTimes"(category);
CREATE EXTENSION btree_gin;
CREATE INDEX sponsor_hash_idx ON "sponsorTimes"("hashedVideoID" COLLATE "C");
CREATE INDEX sponsor_hidden_idx ON "sponsorTimes"("hidden", "shadowHidden");
CREATE INDEX sponsor_votes_idx ON "sponsorTimes"("votes");
