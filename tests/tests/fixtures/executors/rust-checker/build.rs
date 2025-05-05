use rand::{rng, RngCore};

fn main(){
    std::fs::write("build_hash", rng().next_u64().to_string()).unwrap();
}