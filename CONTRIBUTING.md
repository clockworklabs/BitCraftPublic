# Contributing to BitCraftPublic

Thank you for your interest in contributing to **BitCraftPublic**, the server-side code of the BitCraft MMO.
We’re excited to open up our technology to the community, and we welcome contributions that improve its correctness, stability, and usability.
That said, to keep things manageable and aligned with our development goals, we have a few ground rules outlined below.

## How it works

This repo contains the code of the BitCraftServer, which we update every time we do a release. As part of our
release automation a new commit will be added containing all the changes that were made since the last release.
This automation will run usually at the same time or right after the release is made available to players.

We welcome contributions in the form of PRs to this repo. If a PR is approved and merged, the update will be
automatically mirrored in our private repo so that it is picked up for the next release.

## What We're Accepting

We are currently accepting **server-side contributions** that fall into the following categories:

**Bug Fixes**
- Fixes for crashes, panics, or incorrect behavior
- Memory leaks or thread safety issues
- Reproducible problems with clean solutions

**User/Developer Experience Improvements**
- Better error messages or logging
- Build system or dependency fixes
- Safer defaults or config improvements

**Security Improvements**

To ensure that exploits are addressed quickly and not abused by other players, we ask that you submit them to us privately using 
[**this form**](https://docs.google.com/forms/d/e/1FAIpQLSdlQMdxjTmUScKeVL9T-rs7lZOU4VrYl80ida0Kb0d38Nah8w/viewform?usp=sharing&ouid=112232658922648279400).

## What We're Not Accepting (Yet)

To keep the scope narrow and our workflows stable, **please avoid the following**:

**Feature Additions**
- We are not currently accepting new gameplay, mechanics, or systems
- Please don't propose roadmap items or “cool ideas” just yet

**Code Refactoring / Style Changes**
- No large-scale formatting or stylistic refactors
- We’re gradually aligning internal and external codebases — changes will come later

**Speculative Work**
- Please avoid "just in case" PRs — changes must be motivated by a real problem

## Contribution Checklist

Before opening a pull request:

- Make sure your change applies only to server-side code
- Include clear motivation (what bug you fixed or what was improved)
- Keep the changes small and focused
- Explain how you verified correctness of the change
- Ensure your code builds by running `spacetime build`
- Add or update comments where needed
- Use consistent formatting (use `rustfmt`)

## How We Handle Contributions

- You open a PR against the `main` branch of `BitCraftPublic`
- Our team triages your PR and decides whether to:
  - Accept it and import it into our internal codebase
  - Request changes
  - Thank you and close it (if it’s out of scope)

We will **not always respond immediately**, but we will do our best to communicate clearly.

If we accept your change:
- We may **re-implement it internally**, or
- Cherry-pick it into our internal mono-repo, from which the next public release will include it

## Legal & Licensing

By submitting a pull request, you agree that your contribution:

- Is your original work
- Is submitted under the license of this repository (see `LICENSE` file)
- May be modified, redistributed, or used as part of BitCraft

## 🙏 Thank You!

Whether you're fixing a bug, improving performance, or just helping us refine the system — thank you!
Open sourcing BitCraft is a big experiment in openness, and we're grateful you're part of it.
