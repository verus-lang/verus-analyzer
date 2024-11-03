verus! {

pub fn spawn<F, Ret>(f: F) -> (handle: JoinHandle<Ret>) where
    F: FnOnce() -> Ret,
    requires
        f.requires(()),
    ensures
        forall|ret: Ret| #[trigger] handle.predicate(ret) ==> f.ensures((), ret),
{
    let res = std::panic::catch_unwind(
        std::panic::AssertUnwindSafe(
            ||
                {
                    let handle = std::thread::spawn(move || f());
                    JoinHandle { handle }
                },
        ),
    );
    match res {
        Ok(res) => res,
        Err(_) => {
            println!("panic on spawn");
            std::process::abort();
        },
    }
}

} // verus!

fn main() { }
