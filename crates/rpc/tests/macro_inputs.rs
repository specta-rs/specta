// This is a compile test

use specta_rpc::router;

// router!(Router); // Compile error
// router!(Router[]); // Compile error
router!(Router[H]);
router!(Router [H, G]);
router!(Router::<A> [H, G]);
router!(Router::<A, B> [H, G]);

router!(Router[H] where H: Clone);
router!(Router [H, G] where H: Clone);
router!(Router::<A> [H, G] where H: Clone);
router!(Router::<A, B> [H, G] where H: Clone);

router!(Router [H, G] where H: Clone, G: Clone);
router!(Router::<A> [H, G] where H: Clone, G: Clone);
router!(Router::<A, B> [H, G] where H: Clone);

// TODO: Where bounds for Router generics
