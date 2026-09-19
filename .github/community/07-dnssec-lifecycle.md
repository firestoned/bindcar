# DNSSEC lifecycle: enable, observe and delegate on a live zone

> **Goal.** Turn DNSSEC on or off for an existing zone, see its signing state,
> and retrieve the DS record needed at the parent — all without destroying and
> recreating the zone.
>
> **Stop condition.** An operator can take a running unsigned zone to fully
> signed and delegated using only the API, and can tell from the API whether
> signing has actually completed.

> **Status:** ⛔ Not started. Verified against `main` @ `5233c0b`:
> `ModifyZoneRequest` (`src/zones_types.rs`) carries only `also_notify`,
> `allow_transfer` and `allow_update` — no `dnssecPolicy`, no `inlineSigning`.
> `rg -li 'nsec3' src/` and `rg -li 'ds_record|delegation_signer' src/` both
> return nothing.

---

**Created:** 2026-09-18
**Author:** Erick Bourgeois
**Follows:** [`01-dnssec-feature-summary.md`](01-dnssec-feature-summary.md)

## Summary

Roadmap 01 shipped DNSSEC *provisioning*: `dnssecPolicy` and `inlineSigning` can
be set when a zone is **created**, and bindcar emits the matching BIND9
directives. What it did not ship is the lifecycle around that — and roadmap 01
is a 📄 reference document, so the gaps have sat in a "Future Work" list that
nothing tracks.

This roadmap promotes the three that have operational consequences.

## The problem, concretely

Roadmap 01 documents the current procedure for enabling DNSSEC on an existing
zone:

> 1. Delete existing zone
> 2. Recreate with DNSSEC fields
> 3. Publish DS records at parent

That is still accurate. For a zone that is serving traffic, it means deleting
the authoritative data, recreating it, and hoping nothing resolves against the
gap. For a platform whose entire premise is declarative, safe DNS management,
"delete it and make a new one" is the wrong answer to "please sign this zone".

## Three gaps

### 1. `ModifyZoneRequest` cannot express DNSSEC

Verified — the struct carries exactly three fields, none of them DNSSEC:

```rust
pub struct ModifyZoneRequest {
    pub also_notify: Option<Vec<String>>,
    pub allow_transfer: Option<Vec<String>>,
    pub allow_update: Option<Vec<String>>,
}
```

So `PATCH`/`PUT` on a zone cannot turn signing on, cannot change the policy, and
cannot turn it off. The API can create a signed zone and can create an unsigned
one; it cannot move a zone between those states.

### 2. There is no way to retrieve the DS record

Signing a zone accomplishes nothing until the DS record is published at the
parent. bindcar creates the keys (via BIND9) but exposes no way to read back the
resulting DS, so the operator has to go around the API — onto the BIND9 host,
into `/var/cache/bind`, and run `dnssec-dsfromkey` by hand.

That breaks the delegation step out of the automated path entirely, which is
precisely the step an operator most wants automated, because getting it wrong
takes the zone dark for validating resolvers.

### 3. Zone status does not report signing state

`GET` on a zone's status says nothing about DNSSEC. After enabling signing there
is no API answer to "is it signed yet?", "which keys are active?", or "when does
the current RRSIG set expire?" — all of which are the questions asked during a
rollout and during an incident.

Key rollover monitoring is the same gap seen over a longer time horizon.

## Approach

**This needs an ADR before any code.** Per `.claude/rules/testing.md` and the
repo's ADR convention, architecturally significant *how* goes through
`docs/adr/NNNN-title.md` first. Three questions in particular are design
decisions, not implementation details:

1. **Is enabling DNSSEC on a live zone safe to do through `rndc modzone`, or
   does it require freeze/thaw or a reload cycle?** This determines whether the
   operation can be atomic from the caller's point of view, and it is the
   question the whole roadmap turns on. It must be answered against the BIND9
   documentation and a live instance, not assumed.

2. **What does *disabling* DNSSEC mean?** Going secure → insecure is not
   symmetric with going insecure → secure: the DS must be withdrawn at the
   parent and the TTL honoured *before* signing stops, or the zone goes bogus
   for validating resolvers. The API may need to refuse the naive version of
   this outright.

3. **Is DS retrieval a zone sub-resource or part of zone status?** Affects the
   route shape and whether it is cached.

Only once those are settled should the task list below be executed.

## Tasks (post-ADR)

- [ ] ADR: DNSSEC lifecycle transitions on live zones.
- [ ] `src/zones_types.rs`: `dnssec_policy` / `inline_signing` on
      `ModifyZoneRequest`, with the transition rules the ADR settles.
- [ ] `src/zones.rs`: apply them in `modify_zone`, rejecting unsafe transitions
      with a specific `ApiError` rather than a generic 400.
- [ ] DS retrieval endpoint (shape per ADR).
- [ ] DNSSEC block on the zone-status response: signed yes/no, active key tags,
      current signature validity window.
- [ ] `src/zones_test.rs`: each permitted transition, and each refused one.
- [ ] Integration test against a real BIND9 with a `dnssec-policy` defined:
      create unsigned → enable → assert DNSKEY/RRSIG appear → read DS → disable.
- [ ] `docs/src/advanced/dnssec.md`: replace the delete-and-recreate procedure.
- [ ] Update [`01-dnssec-feature-summary.md`](01-dnssec-feature-summary.md)'s
      "Future Work" list to point here.
- [ ] `.claude/CHANGELOG.md`.

## Out of scope

- **NSEC3 configuration.** Listed in roadmap 01's future work; it is a zone
  *configuration* option and belongs with the zone-config surface, not the
  lifecycle. Worth its own item if it is wanted.
- **Automating publication at the parent.** bindcar should *expose* the DS;
  getting it into the parent zone is the caller's business — it may not even be
  a zone bindcar manages.
- **Key management policy.** BIND9 owns key generation and rollover via
  `dnssec-policy`. This roadmap reports on that; it does not reimplement it.

## Definition of done

- A running unsigned zone can be signed through the API alone, and the change is
  observable through the API.
- The DS record for a signed zone is retrievable through the API.
- Unsafe transitions are refused with an error that says what would break.
- No zone deletion anywhere in the documented procedure.
