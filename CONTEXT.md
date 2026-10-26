# Movies Diary

A personal movie diary where users record each watching of a movie with a rating and an optional written review; reviews appear in diaries and social feeds shared with local and federated followers.

## Language

**Review**:
The record of a single watching of a movie: a rating, an optional written comment, when it was watched, and optionally how.
_Avoid_: Diary entry, post

**Diary Entry**:
A Review together with the Movie it is about, as it appears in a diary or a feed.
_Avoid_: Review (that is the record itself)

**Spoiler Flag**:
An author-set marker on a Review stating that its entire comment reveals plot details.
_Avoid_: Spoiler warning, spoiler tag, spoiler markup

**Account deletion**:
Two-phase removal of a user. The request tombstones the account immediately; a background purge then erases its data and notifies the fediverse.
_Avoid_: Account removal, deactivation

**Tombstone**:
The data-free users row (plus signing keypair) an Account deletion leaves behind so queued fediverse `Delete` deliveries can still be signed; removed after a retention period.
_Avoid_: Soft delete

**Retired Username**:
A username of a deleted account, refused for every future registration.
_Avoid_: Reserved username, banned username
