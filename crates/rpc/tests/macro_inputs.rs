// This is a compile test

use specta_rpc::router;

router!(Router::<H>);
router!(Router1::<H> where H: Clone);
router!(Router2::<H, M> where H: Clone);
router!(Router3::<H, M> where H: Clone, M: Clone);
