# Copy Info Design

Authors: [Daniel Ploch](mailto:dploch@google.com)

**Summary:** This Document documents an approach to tracking copy information in
jj repos, in a way that is compatible with both Git's model and with custom
backends that have more complicated explicit tracking of copy information. This
design affects the output of diff commands as well as the results of rebasing
across remote copies.

## Objective

Implement extensible APIs for recording and retrieving copy info for the
purposes of diffing and rebasing across renames and copies more accurately.
This should be performant both for Git, which synthesizes copy info on the fly
between arbitrary trees, and for custom extensions which may explicitly record
and re-serve copy info over arbitrarily large commit ranges.

The APIs should be defined in a way that makes it easy for custom backends to
ignore copy info entirely until they are ready to implement it.

### Read API

Copy information will be served by a single new Backend trait method:

```rust
/// An individual copy source.
pub struct CopySource {
    /// The source path a target was copied from.
    /// It is not requires that the source path is different than the target
    /// path. A custom backend may choose to represent 'rollbacks' as copies
    /// from a file unto itself, from a specific prior commit.
    path: RepoPathBuf,
    /// The source commit the target was copied from. If not specified, then the
    /// parent of the target commit is the source commit.
    id: Option<CommitId>,
    // TODO: Record the FileId here?
}

/// An individual copy event, from file A -> B.
pub struct CopyRecord {
    /// The destination of the copy, B.
    target: RepoPathBuf,
    /// The CommitId where the copy took place.
    id: CommitId,
    /// The source of the copy, A.
    source: CopySource,
}

/// Backend options for fetching copy records.
pub struct CopyRecordOpts {
    /// If true, follow transitive copy records. That is, if file A is copied to
    /// B and then to C, a request for copy records of file C should also return
    /// the copy from A to B.
    transitive: bool
    // TODO: Probably something for git similarity detection
}

pub trait Backend {
    /// Get all copy records for `paths` in the dag range `roots..heads`.
    fn get_copy_records(&self, paths: &[RepoPathBuf], heads: &[CommitId], roots: &[CommitId]) -> BackendResult<Box<dyn Iterator<Item = BackendResult<CopyRecord>> + '_>>;
}
```

Obtaining copy records for a single commit requires first computing the files
list for that commit, then calling get_copy_records with `heads = [id]` and
`roots = parents()`. This enables commands like `jj diff` to produce better
diffs that take copy sources into account.

### Write API

Backends that support explicit copy records at the commit level will do so
through a new field on `backend::Commit` objects:

```rust
pub struct Commit {
    ...
    copies: Option<HashMap<RepoPathBuf, CopySource>>,
}

pub trait Backend {
    /// Whether this backend supports explicit copy records on write.
    fn supports_explicit_copies(&self) -> bool;
}
```

This field will be write-only. Backends may choose to fill it on read, but have
no obligation to do so, as clients are expected to use the `get_copy_records()`
API to get copy info instead. A backend which does not support explicit copy
info may choose to silently ignore it or return an error.

This API will enable the creation of new `jj` commands for recording copies:

```shell
jj cp $SRC $DEST [OPTIONS]
jj mv $SRC $DEST [OPTIONS]
```

These commands will perform the filesystem operations associated with
copies/moves, as well as editing the working copy commit to include records of
these copies.

Flags for both commands will include:

```
-f
    force overwrite the destination path
--after
    record the copy retroactively, without modifying the filesystem
--from REV
    specify a commit id for the copy source that isn't the parent commit
```

For backends which do not support explicit copies, it will be an error to use
`--after`, since this has no effect on anything and the user should know that.

### Rebase Changes

A well known and thorny problem in Mercurial occurs in the following scenario:

1.  Create a new file A
1.  Create new commits on top that make changes to file A
1.  Whoops, I should rename file A to B. Do so, amend the first commit.
1.  Because the first commit created file A, there is no rename to record; it's changing to a commit that instead creates file B.
1.  All child commits get sad on evolve

In jj, we have an opportunity to fix this because all rebasing occurs atomically
and transactionally within memory. The exact implementation of this is yet to be
determined, but conceptually the following should produce desirable results:

1.  Rebase a commit A from parents [B] to parents [C]
1.  Get copy records from [D]->[B] and [D]->[C], where [D] are the common ancestors of [B] and [C]
1.  DescendantRebaser maintains an in-memory map of commits to extra copy info, which it may inject into (2). When squashing a rename of a newly created file into the commit that creates that file, DescendentRebase will return this rename for all rebases of descendants of the newly modified commit. The rename lives ephemerally in memory and has no persistence after the rebase completes.
1.  A to-be-determined algorithm diffs the copy records between [D]->[B] and [D]->[C] in order to make changes to the rebased commit. This results in edits to renamed files being propagated to those renamed files, and avoiding conflicts on the deletion of their sources. A copy/move may also be undone in this way; abandoning a commit which renames A->B should move all descendant edits of B back into A.

In general, we want to support the following use cases:

-   A rebase of an edited file A across a rename of A->B should transparently move the edits to B.
-   A rebase of an edited file A across a copy from A->B should _optionally_ copy the edits to B. A configuration option should be defined to enable/disable this behavior.
-   TODO: Others?

## Non-goals

### Storing explicit copy records in Git

Git uses implicit copy records, generating them on the fly between two arbitrary
trees. It does not have any place for explicit copy info that _exchanges_ with
other users of the same git repo, so any enhancements jj adds here would be
local only and could potentially introduce confusion when collaborating with
other users.

### Directory copies/moves

All copy/move information will be read and written at the file level. While
`jj cp|mv` may accept directory paths as a convenience and perform the
appropriate filesystem operations, the renames will be recorded at the file
level, one for each copied/moved file.
