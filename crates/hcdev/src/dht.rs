use kitsune_p2p_dht::{spacetime::Topology, Arq, ArqBounds, ArqStrat};
use kitsune_p2p_dht_arc::DhtArc;
use structopt::StructOpt;

#[derive(Debug, StructOpt, Clone)]
pub enum Dht {
    Location(DhtLocation),
    Arc(DhtArcSub),
}

impl Dht {
    /// Run the command
    pub fn run(&self) -> anyhow::Result<()> {
        match self {
            Self::Location(cmd) => cmd.run(),
            Self::Arc(cmd) => cmd.run(),
        }
    }
}

#[derive(Debug, StructOpt, Clone)]
pub struct DhtLocation {}

impl DhtLocation {
    pub fn run(&self) -> anyhow::Result<()> {
        unimplemented!()
    }
}

#[derive(Debug, StructOpt, Clone)]
pub struct DhtArcSub {
    input: Vec<u32>,

    /// Convert a continuous arc to quantized coordinates (default)
    #[structopt(short, long)]
    continuous: bool,

    /// Convert quantized coordinates to a continuous arc
    #[structopt(short, long, conflicts_with = "location")]
    quantized: bool,
}

impl DhtArcSub {
    pub fn run(&self) -> anyhow::Result<()> {
        let topo = Topology::standard_zero();
        if self.quantized && !self.continuous {
            if self.input.len() != 3 {
                return Err(anyhow::anyhow!(
                    "quantized input must be three numbers: power, start, and count"
                ));
            }
            let power: u8 = self.input[0].try_into()?;
            let start = self.input[1];
            let count = self.input[2];
            let arq = ArqBounds::new(power.into(), start.into(), count.into());
            println!("{:?}", arq.to_dht_arc_range(&topo));
        } else {
            if self.input.len() != 2 {
                return Err(anyhow::anyhow!(
                    "continuous arc must be a start and an end point"
                ));
            }
            let start = self.input[0];
            let end = self.input[1];
            let arq = Arq::from_dht_arc_approximate(
                &topo,
                &ArqStrat::default(),
                &DhtArc::from_bounds(start, end),
            )
            .to_bounds(&topo);
            println!("{:?}", arq);
        }
        Ok(())
    }
}
