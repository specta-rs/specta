// This is a compile time test of various input combinations

use specta_rpc::router;

use std::fmt::Debug;

// router!(Router); // Compile error
// router!(Router[]); // Compile error
router!(Router1[H]);
router!(Router2 [H, G]);
router!(Router3::<A> [H, G]);
router!(Router4::<A, B> [H, G]);

router!(Router5[H] where H: Clone);
router!(Router6[H] where H: Clone + Debug);
router!(Router7 [H, G] where H: Clone);
router!(Router8::<A> [H, G] where H: Clone);
router!(Router9::<A, B> [H, G] where H: Clone);

router!(Router10 [H, G] where H: Clone, G: Clone);
router!(Router11 [H, G] where H: Clone + Debug, G: Clone + Debug);
router!(Router12::<A> [H, G] where H: Clone, G: Clone);
router!(Router13::<A, B> [H, G] where H: Clone);

router!(Router14::<A, B> where A: Clone, B: Clone; [H]);
router!(Router15::<A, B> where A: Clone + Debug, B: Clone; [H]);
router!(Router16::<A, B> where A: Clone, B: Clone; [H, G]);
router!(Router17::<A, B> where A: Clone, B: Clone; [H, G] where H: Clone);

router!(Router18[H] where H: std::clone::Clone);
router!(Router19::<A, B> where A: std::clone::Clone; [H]);
router!(Router20::<A, B> where A: std::clone::Clone + std::fmt::Debug; [H]);
