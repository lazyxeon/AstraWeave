# astraweave-camera

Canonical camera types for the AstraWeave engine. See
[`docs/current/CAMERA_CONVENTIONS.md`](../docs/current/CAMERA_CONVENTIONS.md)
for the authoritative convention reference.

## Types

- [`Projection`] — perspective projection with derived matrix and original
  parameters (fovy, aspect, znear, zfar). See §2.1, §2.2, §2.3, §2.6 of the
  conventions doc.
- [`RenderView`] — the canonical upload contract from camera producers to the
  renderer. Carries forward and inverse matrices precomputed, position, view
  direction, and projection parameters. See §2.9.
- [`CameraProducer`] — minimal trait every camera producer implements; one
  method `to_render_view()`. See §2.9.

## Status

C.2 (Unified Camera campaign sub-phase 2): types and trait exist; no
production migration yet. C.3 migrates engine `Camera` + `CameraController`
(the FreeFly producer); C.4 migrates editor `OrbitCamera`; C.5–C.7 handle
remaining implementations per the `CAMERA_CONVENTIONS.md` §3 migration
tracking table.

## Dependencies

`glam` only (plus optional `serde` behind a feature flag). No `wgpu`
coupling — this crate produces pure math types; GPU upload happens in the
renderer crate.
