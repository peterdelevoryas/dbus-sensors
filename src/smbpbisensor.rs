use anyhow::Context;
use clap::Parser;
use i2cdev::core::I2CMessage;
use i2cdev::core::I2CTransfer;
use i2cdev::linux::LinuxI2CDevice;

const MAX_POST_BOX_SIZE: usize = std::mem::size_of::<u64>();

/// Smbus post-box interface sensor management
#[derive(Debug, clap::Parser)]
struct Args {
    #[command(subcommand)]
    subcommand: Subcommand,
}

#[derive(Debug, clap::Subcommand)]
enum Subcommand {
    /// Read a post-box
    Read {
        /// Post-box interface I2C bus index
        #[arg(long)]
        bus: u8,

        /// Post-box interface I2C address
        #[arg(long)]
        address: u16,

        /// Post-box offset
        #[arg(long)]
        offset: u16,

        /// Post-box size
        #[arg(long)]
        size: usize,
    },
}

trait SmbusPostBoxInterface {
    fn smbus_read_post_box(&mut self, offset: u16, out: &mut [u8]) -> anyhow::Result<()>;
}

impl<T> SmbusPostBoxInterface for T
where
    for<'a> T: I2CTransfer<'a>,
    for<'a> <T as I2CTransfer<'a>>::Error: std::error::Error + Send + Sync + 'static,
{
    fn smbus_read_post_box(&mut self, offset: u16, out: &mut [u8]) -> anyhow::Result<()> {
        let mut buf = [0_u8; std::mem::size_of::<u16>()];
        let offset = if offset <= u8::MAX as u16 {
            buf[0] = offset as u8;
            &buf[..=0]
        } else {
            buf = offset.to_be_bytes();
            &buf[..]
        };
        let mut messages = [T::Message::write(offset), T::Message::read(out)];
        let m = messages.len();
        let n = self
            .transfer(&mut messages)
            .context("Unable to complete i2c transfer")?;
        anyhow::ensure!(
            n as u64 == m as u64,
            "Only {n}/{m} messages were transmitted successfully "
        );
        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    match args.subcommand {
        Subcommand::Read {
            bus,
            address,
            offset,
            size,
        } => {
            let bus_path = format!("/dev/i2c-{bus}");
            let mut i2c = LinuxI2CDevice::new(&bus_path, address)
                .with_context(|| format!("Unable to open {bus_path} @{address}"))?;

            let mut value = [0_u8; MAX_POST_BOX_SIZE];
            anyhow::ensure!(
                size <= MAX_POST_BOX_SIZE,
                "Maximum post-box size if {MAX_POST_BOX_SIZE} bytes"
            );

            i2c.smbus_read_post_box(offset, &mut value[..size])
                .with_context(|| format!("Unable to read post-box at +{offset}, size={size}"))?;

            println!("{value:#02x?}");
        }
    }
    Ok(())
}
