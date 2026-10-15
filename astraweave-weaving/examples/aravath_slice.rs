fn main() {
    let slice = astraweave_weaving::run_aravath_slice();
    println!(
        "aravath sword hp {} weave hp {} cover intact {} cover damage {}",
        slice.sword_hp, slice.weave_hp, slice.cover_intact, slice.cover_damage
    );
    assert_eq!(slice.sword_hp, 15);
    assert_eq!(slice.weave_hp, 25);
    assert!(!slice.cover_intact);
}
