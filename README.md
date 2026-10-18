# benchmark-data

History store for `benchmark-action/github-action-benchmark`. This branch is not the GitHub Pages site.

The Pages site stays on `gh-pages`. `main` also contains a `gh-pages/` directory, so `git checkout gh-pages` after cloning `main` is ambiguous and the action cannot use that branch name.

`dev/bench/data.js` was seeded from the combined artifact of run 39118239968 (PR head `b6811ec4`). That run is the previous suite the PR comment step compares against. There was no earlier `data.js` on `gh-pages`.
