# Jujutsu from first principles

## Preface

Why does Jujutsu exist and which problems does it solve? This document tries to
answer both of these questions while expanding on the design in a user-friendly
way. 

At its core Jujutsu is [Version Control System][vcs] which scales to huge 
repositories at [Google scale][billion-lines]. Many design choices are 
influenced by the concurrent commits happening in Googles Monorepo, as there 
are always multiple people working on the same file(s) at the same time.

## Core Tenets

Jujutsu's core tenets are:

 1. Having as few states as possible.
 1. Make it fundamentally impossible to lose work in your repository.
 1. Allow concurrent edits on any commit, pending or finished.
 1. Make a "stacked diffs" workflow as easy as possible.

## Base design

The initial base design is to be a conceptually simpler Mercurial, as 
automatically snapshotting the working copy simplifies the UX of the 
command-line interface by a huge amount and avoids many bad states.

By also choosing to operate by default on the history of the repository (
just called the "the Graph" from now on) instead of files, all history 
modifying commands can be done at any point. This is a major improvement on 
other version control systems as they need to re-apply a single patch on each 
new ancestor before finishing the Graph rewrite. Since the Graph can be changed
at any point, the working copy cannot contain any state depending on it, thus 
we have the working-copy commit, which just is another commit from the Graph's
point of view. 


### Change-IDs and Changes

Since Jujutsu is oriented around a "stacked diffs" kind of workflow, which 
primarily work on individually versiond patch sets, some kind of container is 
needed, this is what a Change is. They are provided with a unique id to address
them easily. 

TODO: expand on change-ids and "working on the graph/repo" instead of a commit


[billion-lines]: https://www.youtube.com/watch?v=W71BTkUbdqE&t=327s
[vcs]: https://en.wikipedia.org/wiki/Version_control 
