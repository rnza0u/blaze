use rand::{thread_rng, RngCore};

fn main(){
    std::fs::write("build_hash", thread_rng().next_u64().to_string()).unwrap();
}