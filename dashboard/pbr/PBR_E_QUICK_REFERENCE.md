# Phase PBR-E: Quick Reference Card

## 🚀 Quick Start (30 seconds)

```powershell
# Run the demo
cargo run -p unified_showcase --release

# Press F5 to enable PBR-E demo
# Press F6 to cycle materials
# Press F7/F8 to adjust grid size
```

---

## ⌨️ Keyboard Controls

| Key | Action | Details |
|-----|--------|---------|
| **F5** | Toggle PBR-E Demo | ON: Show sphere grid / OFF: Show terrain |
| **F6** | Cycle Material Type | Clearcoat → Anisotropy → Subsurface → Sheen → Transmission |
| **F7** | Decrease Grid Size | Min 3x3 (9 spheres) |
| **F8** | Increase Grid Size | Max 10x10 (100 spheres) |

---

## 🎨 Material Types

| Material | Parameter X | Parameter Y | Visual Effect |
|----------|-------------|-------------|---------------|
| **Clearcoat** | Strength (0→1) | Roughness (0→1) | Car paint, lacquer, dual specular lobes |
| **Anisotropy** | Strength (-1→1) | Rotation (0→2π) | Brushed metal, hair, elliptical highlights |
| **Subsurface** | Scale (0→1) | Radius (0→5mm) | Skin, wax, soft translucent |
| **Sheen** | Intensity (0→1) | Roughness (0→1) | Velvet, fabric, edge glow |
| **Transmission** | Factor (0→1) | IOR (1.0→2.5) | Glass, water, transparent |

---

## ✅ Testing Checklist (5 minutes)

- [ ] Build release: `cargo build -p unified_showcase --release`
- [ ] Run application: `cargo run -p unified_showcase --release`
- [ ] Press F5 → Verify 25 spheres appear
- [ ] Press F6 5 times → Verify all materials cycle
- [ ] Press F8 5 times → Verify grid grows to 10x10
- [ ] Check FPS ≥30 with 100 spheres
- [ ] Capture screenshot of each material

---

## 📊 Performance Targets

| Grid Size | Spheres | Target FPS | Expected FPS (RTX 3060 Ti) |
|-----------|---------|------------|----------------------------|
| 3x3       | 9       | >100 FPS   | 200-400 FPS                |
| 5x5       | 25      | >60 FPS    | 100-200 FPS                |
| 8x8       | 64      | >40 FPS    | 60-120 FPS                 |
| **10x10** | **100** | **>30 FPS** | **40-80 FPS (CRITICAL)** |

---

## 📁 Key Files

| File | Purpose | Lines | Status |
|------|---------|-------|--------|
| `examples/unified_showcase/src/pbr_e_demo.rs` | Demo scene generation | ~250 | ✅ Complete |
| `examples/unified_showcase/src/enhanced_shader.wgsl` | WGSL shader (SSBO, eval) | +120 | ✅ Complete |
| `examples/unified_showcase/src/main.rs` | Renderer + UI controls | +210 | ✅ Complete |
| `PBR_E_TESTING_GUIDE.md` | Comprehensive testing | ~2000 | ✅ Complete |
| `PBR_E_COMPLETE_INTEGRATION_TESTING_SUMMARY.md` | Final summary | ~1200 | ✅ Complete |

---

## 🐛 Troubleshooting

| Issue | Solution |
|-------|----------|
| **No spheres visible** | Check console for errors, verify F5 pressed, try F8 to increase grid |
| **Black spheres** | SSBO binding issue - check bind group 6 is set in render pass |
| **Low FPS (<30)** | Use `--release` flag (CRITICAL), reduce grid size with F7 |
| **Crash on F5** | Check material_id range, verify SSBO buffer size correct |

---

## 📸 Screenshot Checklist

Required (5 total):
- [ ] `unified_showcase_pbr_e_clearcoat.png` (5x5 grid)
- [ ] `unified_showcase_pbr_e_anisotropy.png` (5x5 grid)
- [ ] `unified_showcase_pbr_e_subsurface.png` (5x5 grid)
- [ ] `unified_showcase_pbr_e_sheen.png` (5x5 grid)
- [ ] `unified_showcase_pbr_e_transmission.png` (5x5 grid)

Camera: Distance 15 units, height 8 units, angle 30-45°

---

## 🎯 Success Criteria

**Implementation**: ✅ 6/6 tasks complete (100%)
**Compilation**: ✅ 0 errors, 1.06s build time
**Documentation**: ✅ 3 docs, 3600+ lines
**Visual Testing**: ⏳ Pending (0/5 materials)
**Performance**: ⏳ Pending (0/4 grid sizes)

**Overall**: 🎉 **Code Complete** | ⏳ **Awaiting Visual Validation**

---

## 📚 Full Documentation

- **Integration Summary**: `PBR_E_INTEGRATION_COMPLETE.md` (~550 lines)
- **Testing Guide**: `PBR_E_TESTING_GUIDE.md` (~2000 lines)
- **Final Summary**: `PBR_E_COMPLETE_INTEGRATION_TESTING_SUMMARY.md` (~1200 lines)
- **Quick Reference**: `PBR_E_QUICK_REFERENCE.md` (this document)

---

## ⏱️ Time Estimates

- **Quick Test**: 5 minutes (verify all 5 materials render)
- **Basic Validation**: 30 minutes (test + 1 screenshot per material)
- **Comprehensive Testing**: 90 minutes (follow full testing guide)
- **Performance Profiling**: 30 minutes (benchmark all grid sizes)

---

**Version**: 1.0  
**Status**: Ready for Testing ⏳  
**Next Step**: `cargo run -p unified_showcase --release` + Press F5
