# Vendored architecture-map libraries

`docs/architecture/workspace_map.html` loads these six files from this directory.
They are the same bytes the page used to load from unpkg. The loader sets each
file's sha384 `integrity` and `crossorigin` unless `location.protocol` is
exactly `file:`. A `file://` open omits both. Every other scheme keeps the
check, so a browser still refuses a file that does not match. The hashes below
were checked against those attributes and against the bytes in this directory.

These were already the page's runtime dependencies. Vendoring removes the
dependency on unpkg being reachable when someone opens the map. Versions are
unchanged. `docs.yml` copies the `.js` files and `licenses/` to
`docs/book/architecture/vendor/` next to the published `index.html`.

There is no seventh library. A small loader inserts these six script tags and
starts `map-viewer.js` only after all six have loaded. Any failure shows a
message and does not start the viewer. Each library tag keeps its sha384
`integrity` and `crossorigin` unless the protocol is exactly `file:`. A direct
`file://` open skips that integrity check, because a file origin cannot satisfy
Subresource Integrity. GitHub Pages and every other scheme enforce it.

`cytoscape.version` reports `3.29.1` even though this file is byte-identical to
unpkg `cytoscape@3.30.0`. Upstream's 3.30.0 build ships that stale version
string.

| File | Version | Source URL | sha384 (SRI, existing attribute) | sha256 |
|---|---|---|---|---|
| `cytoscape-3.30.0.min.js` | 3.30.0 | https://unpkg.com/cytoscape@3.30.0/dist/cytoscape.min.js | `kpMsYllYzyaWU69Piok08rPNktpnjqAoDMdB00fjqUkEk3lkuUbSuwJ+oXrjvN6B` | `a298b253e4b9b08bd0b2fe222ad67b2b9f42d057d7c17f7a050512079c46fddd` |
| `layout-base-2.0.1.js` | 2.0.1 | https://unpkg.com/layout-base@2.0.1/layout-base.js | `5E2lB9AIGE6LRCnOOSTnZRlYZFZ01iMeN2fw97Z1r4Z/kXALxKw2AC+ZzQqoeDsG` | `ec15ab5df9af3f20708f4faab994accf91cda71848cd5bb10a23432cc50b6745` |
| `cose-base-2.2.0.js` | 2.2.0 | https://unpkg.com/cose-base@2.2.0/cose-base.js | `RswRBkrMsPUYpJLbZ1CVA08zbNzAkykE2oGJTujBwfjWNdfxv2WVjLJNqv1LhAOp` | `7cae9509bd36235a63a85e71c8d9fa2cd0bc1d0c1ecc5b5a737976f39d040ddf` |
| `cytoscape-fcose-2.2.0.js` | 2.2.0 | https://unpkg.com/cytoscape-fcose@2.2.0/cytoscape-fcose.js | `uk5Wbjq1+KqUdHO30w7N7GrEGdzBhaJeW9o/ANF6v9+yx3M/cBmoX51C000JNCUf` | `4b1cab218d74996aa59cd8473f9239cc6398b8c1774d84d7e59ad9a68959cb57` |
| `dagre-0.8.5.min.js` | 0.8.5 | https://unpkg.com/dagre@0.8.5/dist/dagre.min.js | `2IH3T69EIKYC4c+RXZifZRvaH5SRUdacJW7j6HtE5rQbvLhKKdawxq6vpIzJ7j9M` | `62eb9787ccfdbdf4148d4d99d31dbf9ee4770eafee81e637d759b52aac22cd51` |
| `cytoscape-dagre-2.5.0.js` | 2.5.0 | https://unpkg.com/cytoscape-dagre@2.5.0/cytoscape-dagre.js | `u69h9ebXeSjlg6q/rb1zKTRAGu/h8deCl0409xpS/QJctMKnc4M9Fzkm01VOQdeF` | `bf70fe402991dcbff33e05a7e4a5271c78020bb75e85d1c80ab7538e4157112e` |

Load order is the order in the table. `cytoscape-fcose` needs `layout-base` and
`cose-base`. `cytoscape-dagre` needs `cytoscape` and `dagre`.

Upstream `LICENSE` files are in `licenses/`. All six packages are MIT.
`cytoscape-3.30.0.min.js` and `dagre-0.8.5.min.js` also carry the MIT notice
inside the file. `layout-base-2.0.1.js` also carries the Apache-2.0 notice for
its JamaJS-derived SVD code (Apache-2.0 text at lines 4326-4527; the JamaJS attribution starts at line 4319). `docs.yml` publishes
`licenses/` next to the scripts so those four bundles are not served without
their grant.

Dagre 0.8.5 does not call `Math.random`. The layered layout still sorts node
ids and edge endpoints before it builds the graph, because dagre's rank and
crossing steps follow insertion order.
