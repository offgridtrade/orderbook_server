use offgrid_primitives::{market::L1, prices::L2};

fn main() -> anyhow::Result<()> {
    let l1 = L1::new(100, 95, 105, 5, 6, 7, 8);
    let l2 = L2::default();

    println!("runtime {} ready", runtime::version());
    println!("L1 bid head: {}", l1.bid_head);
    println!("L2 levels -> bids: {}, asks: {}", l2.bids.len(), l2.asks.len());

    Ok(())
}
