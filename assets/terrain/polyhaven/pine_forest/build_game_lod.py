#!/usr/bin/env python3
"""Offline LOD build for the forest opening plants.

Pinned tool: gltfpack 1.3
  url: https://github.com/zeux/meshoptimizer/releases/download/v1.3/gltfpack-ubuntu.zip
  zip sha256: 0666d9dc40d60fe5b9a45f3fc24f8e6ca87112974bd5de9ca57152aa13d06017
  binary sha256: 595441045457bf3d069ec14fa807b1034d324db003bd66ac6377977cc11e7da3

The photogrammetry .bin buffers are build inputs. They are not committed.
Download each next to its 1k glTF, then run, from that directory:

  gltfpack -i <id>_1k.gltf -o <id>_lod0.glb -sa -si <lod0> -tc -vp 14 -vt 12 -vn 8
  gltfpack -i <id>_1k.gltf -o <id>_lod1.glb -sa -si <lod1> -tc -vp 14 -vt 12 -vn 8
  gltfpack -i <id>_1k.gltf -o <id>_lod2.glb -sa -si <lod2> -tc -vp 14 -vt 12 -vn 8

-sa is required. A 1% error cap cannot reach 15k triangles from these
sources. -tc writes KTX2. Two runs of this binary write identical bytes.

Ratios (target triangles / source triangles):

  pine_tree_01              17182252 tris  lod0 0.000873  lod1 0.000291  lod2 0.000087
  tree_small_02              2062487 tris  lod0 0.007273  lod1 0.002424  lod2 0.000727
  island_tree_01             1599403 tris  lod0 0.009378  lod1 0.003126  lod2 0.000938
  dry_branches_medium_01       16803 tris  lod0 0.892697  lod1 0.297565  lod2 0.089271

The billboard is a two-quad cross (4 triangles) written by this repo's
generator and packed with:

  gltfpack -i billboard.gltf -o <id>_billboard.glb -tc -vp 14 -vt 12 -vn 8

Outputs live in game/ and their SHA-256 is in manifest.json.
"""
