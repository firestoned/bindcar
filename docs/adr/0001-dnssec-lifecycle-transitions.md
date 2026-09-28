<!--
Copyright (c) 2026 Erick Bourgeois, firestoned
SPDX-License-Identifier: MIT
-->
# 0001 — DNSSEC lifecycle transitions on live zones

- **Status:** Accepted
- **Date:** 2026-09-27
- **Proposed:** 2026-09-27
- **Deciders:** Erick Bourgeois
- **Related:** Execution plan:
  [roadmap 07](../../.github/community/07-dnssec-lifecycle.md). Downstream
  consumer: bindy
  [ADR-0006](https://github.com/firestoned/bindy/blob/main/docs/adr/0006-dnssec-ds-record-status-reporting.md)
  (its `DNSZone.status.dnssec.nextKeyRollover`/`lastKeyRollover` stay null
  until the status surface decided here ships). Implemented 2026-09-27,
  immediately after acceptance; the roadmap tracks the task list.

## Context

Roadmap 07 promotes three gaps left by roadmap 01: `ModifyZoneRequest` cannot
express DNSSEC, the DS record is not retrievable through the API, and zone
status says nothing about signing state. It required an ADR to settle three
design questions before any code:

1. Is enabling DNSSEC on a live zone safe through `rndc modzone`, or does it
   require freeze/thaw or a reload cycle?
2. What does *disabling* DNSSEC mean, given that secure → insecure is not
   symmetric with insecure → secure?
3. Is DS retrieval a zone sub-resource or part of zone status?

The roadmap demanded these be answered against the BIND9 documentation **and a
live instance**, not assumed.

### Evidence: live verification (BIND 9.18.50)

Verified 2026-09-27 against `internetsystemsconsortium/bind9:9.18`
(BIND 9.18.50 Extended Support Version), running with `allow-new-zones yes`
and a zone added via `rndc addzone` — the exact shape bindcar manages. The
zone answered queries throughout every transition below; no query loss was
observed at any step.

| Action on a running zone | Result |
|---|---|
| `rndc modzone` adding `dnssec-policy` + `inline-signing yes` | Config **stored** (NZD updated, `showzone` reflects it) but **not applied**: `rndc dnssec -status` still reports "Zone does not have dnssec-policy"; no keys generated |
| `rndc reload <zone>` after that modzone | Still not applied ("zone reload up-to-date") |
| `rndc sign <zone>` after that modzone | Fails (`'sign' failed: permission denied` — the running zone is not a kasp zone yet) |
| `rndc reconfig` (or global `rndc reload`) after that modzone | Policy applied: CSK generated within seconds, zone signed inline, DNSKEY/RRSIG served, key `.state` file written |
| `rndc modzone` changing an unrelated option while **keeping** the `dnssec-policy`/`inline-signing` directives | Signing undisturbed |
| `rndc modzone` **omitting** `dnssec-policy` on a signed zone, then `reconfig` | Zone **abruptly unsigned**: DNSKEY and RRSIGs removed immediately, no grace period, no refusal, no warning |
| `rndc modzone` switching to the built-in `dnssec-policy "insecure"`, then `reconfig` | Graceful teardown managed by named: key goal moves to `hidden`, DNSKEY/RRSIG lifecycle states (`unretentive`) tracked through removal. (In this test the DS state was `hidden` — never published at a parent — so removal was immediate; with a published DS, named retains the chain until DS withdrawal is confirmed) |

Documentation (BIND 9.18 ARM, DNSSEC Guide "Reverting to Unsigned"):

> The "insecure" policy is a built-in policy (like "default"). It makes sure
> the zone is still DNSSEC-maintained, to allow for a graceful transition to
> unsigned. It also publishes the CDS and CDNSKEY DELETE records automatically
> at the appropriate time. […] When the DS records have been removed from the
> parent zone, use `rndc dnssec -checkds -key id withdrawn example.com` […]
> After a while, the zone is reverted back to the traditional, insecure DNS
> format. This can be verified by checking that all DNSKEY and RRSIG records
> have been removed from the zone. The dnssec-policy line can then be removed
> from named.conf and the zone reloaded.

And on the prerequisites:

> The `dnssec-policy` statement requires dynamic DNS to be set up, or
> `inline-signing` to be enabled.

Also verified: `rndc dnssec -status <zone>` reports the policy name, each
key's tag, algorithm, role (KSK/ZSK/CSK), per-facet state
(dnskey/ds/zone rrsig/key rrsig: `hidden`/`rumoured`/`omnipresent`/
`unretentive`), the `since` timestamp for published/key-signing/zone-signing,
and rollover scheduling ("No rollover scheduled" / next event). The key
`.state` file additionally carries explicit `Generated`/`Published`/`Active`/
`PublishCDS`/`*Change` timestamps. The public key file
`K<zone>.+<alg>+<tag>.key` contains the DNSKEY RR in standard textual form.

### Constraints from the existing code

- `modify_zone` (`src/zones.rs`) round-trips the zone config:
  `rndc showzone` → `parse_showzone` → mutate → `to_rndc_block` →
  `rndc modzone`. Unknown directives — including `dnssec-policy` today —
  survive only because the parser keeps them in `ZoneConfig::raw_options` and
  `to_rndc_block` re-emits them. There is no typed `dnssec_policy` field.
- `RndcClient` (`src/rndc.rs`) has no `reconfig` method.
- Zone creation already renders `dnssec-policy`/`inline-signing`
  (roadmap 01), so the create path and the modify path must accept the same
  fields with the same validation (`validate_rndc_identifier`).
- The record path was reverted to the `nsupdate` binary (no hickory), so
  there is no in-process DNS client to query DNSKEY over the wire.

## Decision

1. **Enable = `modzone` + `reconfig`, no freeze/thaw.**

   Enabling DNSSEC on a live zone is done by adding `dnssec-policy "<name>"`
   and `inline-signing yes` to the zone config via `rndc modzone`, then
   issuing `rndc reconfig` to activate it. This is safe and non-disruptive —
   but it is **not sufficient to modzone alone**: verified on 9.18.50,
   `modzone` persists the policy without applying it, and neither a per-zone
   reload nor `rndc sign` activates it.

   - `RndcClient` gains a `reconfig()` method. The DNSSEC transition path in
     `modify_zone` calls `modzone` then `reconfig`; the operation is atomic
     from the caller's point of view in the sense that the config change is
     persisted first and activation follows within the same request. If
     `reconfig` fails after a successful `modzone`, the response must say the
     zone config was stored but not yet active (named will apply it on its
     next reconfig/restart) — not report a clean failure.
   - `inline-signing yes` is set implicitly whenever `dnssec_policy` is set
     and the zone has no `allow-update`/`update-policy` (the ARM requires one
     or the other). bindcar-managed zones use inline signing; the unsigned
     zone file is never rewritten by named, so `rndc freeze`/`thaw` is not
     involved — those exist for hand-editing dynamic zone files, which this
     flow never does.
   - `reconfig` is global to the named instance. That is acceptable: it only
     loads new/changed configuration and does not re-transfer or bounce
     unaffected zones (verified: other zones kept serving). DNSSEC
     transitions are serialized behind the existing per-instance rndc client,
     so concurrent PATCHes cannot interleave modzone/reconfig pairs.

2. **PATCH semantics: omission is "no change"; disabling is a guarded
   two-phase transition.**

   `ModifyZoneRequest` gains `dnssec_policy: Option<String>` and
   `inline_signing: Option<bool>` with **merge semantics**: a field that is
   absent from the PATCH body means "leave as is", never "remove". This is
   load-bearing: verified live, re-issuing a zone config without
   `dnssec-policy` abruptly unsigns the zone at the next reconfig — DNSKEY
   and RRSIG gone immediately. With a DS still published at the parent, that
   takes the zone dark for every validating resolver. Therefore:

   - The typed `dnssec_policy` field is parsed out of `showzone` output into
     `ZoneConfig` (promoted from `raw_options` to a first-class field), so
     the round-trip can never drop it by accident.
   - **Going secure → insecure is only expressible as
     `"dnssecPolicy": "insecure"`** — the built-in policy makes BIND manage
     the graceful teardown (CDS/CDNSKEY DELETE publication, retention until
     DS withdrawal). This is the officially documented reversion path.
   - A request that would *remove* the `dnssec-policy` directive from a zone
     that still serves DNSKEY records is **refused** with a specific
     `ApiError::UnsafeDnssecTransition` explaining that the zone must first
     transition through the `insecure` policy and complete DS withdrawal.
     Removing the directive is permitted only once the zone no longer serves
     DNSKEY/RRSIG (the "reverted" state in the official procedure).
   - Switching between two real policies (e.g. algorithm rollover) is passed
     through to BIND, which manages the rollover; bindcar validates only that
     the policy name is a valid rndc identifier. bindcar does not attempt to
     enumerate the policies defined in `named.conf` — a nonexistent policy
     name fails at `reconfig` with named's own error, which is surfaced
     verbatim.
   - DS withdrawal confirmation (`rndc dnssec -checkds -key <id> withdrawn`)
     is exposed as its own endpoint so the insecure transition can complete
     without host access when no parental agents are configured. (Endpoint
     shape: `POST /api/v1/zones/{name}/dnssec/checkds` with
     `{"keyTag": <u16>, "ds": "published"|"withdrawn"}`.)

3. **DS is a zone sub-resource; key timing comes from
   `rndc dnssec -status`.**

   - **DS retrieval is a zone sub-resource**:
     `GET /api/v1/zones/{name}/ds`, computed on demand, not cached. It is
     delegation material an operator copies to the parent/registrar at a
     specific moment; burying it in status invites staleness, and status
     must stay cheap to poll.
   - The DNSKEY RDATA needed to compute the DS is read from the **public**
     key files (`K<zone>.+<alg>+<tag>.key`, standard RR text) in the zone's
     key-directory, filtered to keys that `rndc dnssec -status` reports in a
     KSK role. The DS digest (SHA-256, digest type 2) and key tag are
     computed in-process (RFC 4034 appendix B / RFC 4509) — no
     `dnssec-dsfromkey` binary in the image, no DNS client dependency. This
     requires the key-directory to be mounted **read-only** into the bindcar
     container; `.private` and `.state` files are never read, and the
     endpoint must refuse to serve anything but computed DS RRs.
   - **The zone-status DNSSEC block is sourced from parsing
     `rndc dnssec -status <zone>`**, not from key `.state` files. Rationale:
     the rndc output is the supported operator interface, reflects the
     *running* server's view (a `.state` file can describe a policy not yet
     activated — exactly the modzone-without-reconfig gap verified above),
     includes rollover scheduling, and needs no new deployment coupling
     beyond the rndc channel bindcar already owns. `.state` files are
     BIND-internal bookkeeping. The block reports: `signed` (yes/no), policy
     name, and per key: tag, algorithm, role, the four facet states, `since`
     timestamps, and next/last rollover event when named reports one. This
     is the surface that unblocks bindy ADR-0006's
     `nextKeyRollover`/`lastKeyRollover`.

4. **Version caveat.** The modzone-does-not-activate behavior is verified on
   BIND 9.18.50 (ESV). The integration test added by roadmap 07 must assert
   the full enable-via-API-then-observe path so that a future BIND (9.20+)
   changing this quirk is caught by CI rather than by code reading.

## Consequences

- Enabling and disabling DNSSEC on a live zone becomes possible through the
  API alone, with no zone deletion and no observed query loss; the
  delete-and-recreate procedure in `docs/src/advanced/dnssec.md` is retired.
- `rndc reconfig` becomes part of bindcar's vocabulary. It is global; any
  independently made pending config change on the instance is activated along
  with the DNSSEC transition. Documented as a known property.
- The DS endpoint couples bindcar's deployment to a read-only mount of the
  key-directory. Deployments that do not mount it get a specific error from
  `GET …/ds`; everything else (enable, disable, status) works without it.
- `dnssec-policy` becomes a typed, round-trip-safe field of `ZoneConfig`;
  the raw_options fallback no longer carries it.
- The API refuses the naive unsigning request outright. Operators who truly
  want an abrupt unsign (lab environments) can still do it out-of-band with
  rndc; bindcar will not offer it.
- bindy ADR-0006's null `nextKeyRollover`/`lastKeyRollover` fields become
  populatable from the new status block.
