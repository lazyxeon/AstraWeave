# Mozilla Public License 2.0 — runevision erosion filter port

The runevision erosion filter implementation in
`astraweave-terrain/src/runevision_erosion.rs` is derived from work by
Rune Skovbo Johansen, originally published as the "Fast and Gorgeous
Erosion Filter" blog post (March 2026):

- Blog: <https://blog.runevision.com/2026/03/fast-and-gorgeous-erosion-filter.html>
- Companion video: <https://www.youtube.com/watch?v=r4V21_uUK8Y>
- Predecessor "Phacelle Cheap Directional Noise": <https://blog.runevision.com/2026/01/phacelle-cheap-directional-noise.html>
- Reference Shadertoy implementation: <https://www.shadertoy.com/view/wXcfWn>

The blog post and accompanying code are licensed under the Mozilla Public
License v2.0. AstraWeave's port is therefore distributed under MPL-2.0
for the file `astraweave-terrain/src/runevision_erosion.rs`. The rest of
AstraWeave's terrain crate remains under the parent project's MIT
license; the MPL-2.0 obligations apply only to this single file and any
modifications thereto.

The full text of the MPL-2.0 is available at:
<https://www.mozilla.org/en-US/MPL/2.0/>

Per MPL-2.0 §1.4, the source code (this file) is publicly available in
the AstraWeave repository. Modifications to `runevision_erosion.rs` must
remain MPL-2.0 licensed; combination with code under other licenses is
permitted per MPL-2.0 §1.10.

## Attribution

Original concept and reference implementation: **Rune Skovbo Johansen**
(<https://blog.runevision.com/>).

The AstraWeave port is a faithful interpretation of the algorithm
described in the blog post. The exact GLSL source from the blog post was
not directly fetched during the porting session (the URL was inaccessible
to the porting agent's environment); the implementation is based on the
research summary captured in
`docs/audits/uber_noise_research_2026-04-25.md` Rank 2 section, which
describes the algorithm structurally (gradient-aligned gully extrusion,
multi-octave mask attenuation via `pow_inv(combiMask, detail) * newMask`,
altitude fade via `inverse_lerp(valley_alt, peak_alt, h) * 2 - 1`, and
the cosine/sine wave gully composition). Specific numeric parameters
and exact math may differ slightly from the canonical reference. If the
implementation produces visibly different output from the blog's
demonstrations, this is documented as a porting interpretation rather
than a faithful direct translation.
