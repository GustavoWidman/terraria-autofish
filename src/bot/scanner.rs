use std::time::Duration;

use eyre::{Result, eyre};
use log::{debug, error, info, warn};
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;

use crate::bot::reader::ProcessReader;
use crate::utils::config::Config;

/// commands that can be sent to the scanner task
#[derive(Debug, Clone)]
pub enum ScannerCommand {
    ZeroMemory,
    TogglePause(bool),
}

pub struct MemoryScanner {
    config: Config,
    reader: ProcessReader,
    ptr: usize,
    last_fish_id: i32,
    cmd_rx: mpsc::UnboundedReceiver<ScannerCommand>,
}

impl MemoryScanner {
    pub fn new(config: Config) -> Result<(Self, mpsc::UnboundedSender<ScannerCommand>)> {
        let reader = ProcessReader::new()?;

        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<ScannerCommand>();
        let mut bot = MemoryScanner {
            config,
            ptr: Self::find_fish_ptr(&reader)?,
            reader,
            last_fish_id: 0,
            cmd_rx,
        };

        bot.last_fish_id = bot
            .poll_fish()
            .map_err(|e| eyre!("initial read failed:\n{:#?}", e))?;

        Ok((bot, cmd_tx))
    }

    fn find_fish_ptr(reader: &ProcessReader) -> Result<usize> {
        info!("scanning for `_context` pointer...");

        // pattern from `Terraria.Projectile.FishingCheck(): void` disassembly:
        //
        // 55                    push ebp
        // 8B EC                 mov  ebp, esp
        // 57                    push edi
        // 56                    push esi
        // 50                    push eax
        // 8B F9                 mov  edi, ecx
        // ;     FishingContext context = Projectile._context;
        // 8B 35 ?? ?? ?? ??     mov  esi, ds:[addr]
        // ;     if (!this.TryBuildFishingContext(context))
        // 8B CF                 mov  ecx, edi
        // 8B D6                 mov  edx, esi
        // ... (not needed for our purposes)

        let pattern = vec![
            Some(0x55), // push ebp
            //
            Some(0x8B),
            Some(0xEC), // mov ebp,esp
            //
            Some(0x57), // push edi
            //
            Some(0x56), // push esi
            //
            Some(0x50), // push eax
            //
            Some(0x8B),
            Some(0xF9), // mov edi,ecx
            //
            Some(0x8B),
            Some(0x35), // mov esi,ds:[addr]
            None,
            None,
            None,
            None, // static field address (wildcard)
            //
            Some(0x8B),
            Some(0xCF), // mov ecx,edi
            //
            Some(0x8B),
            Some(0xD6), // mov edx,esi
        ];

        let instruction_addr = reader
            .pattern_scan(0x10000000, 0x40000000, &pattern)
            .map_err(|e| eyre!("pattern scan failed:\n{:#?}", e))?;

        debug!("found pattern at 0x{:X}", instruction_addr);

        let static_addr = reader
            .read_memory(instruction_addr + 10)
            .map(|addr: u32| addr as usize)?;

        debug!("static field address at 0x{:X}", static_addr);

        let ptr = reader
            .read_memory(static_addr)
            .map(|ptr: u32| (ptr + 0x68) as usize)?;

        debug!("final pointer address at 0x{:X}", ptr);

        info!("successfully hooked into fishing context!");

        Ok(ptr)
    }

    fn poll_fish(&mut self) -> Result<i32> {
        let fish_id: i32 = self.reader.read_memory(self.ptr)?;

        Ok(fish_id)
    }

    fn zero_memory(&self) -> Result<()> {
        self.reader.write_memory(self.ptr, &0i32)?;
        debug!("zeroed fish memory at 0x{:X}", self.ptr);
        Ok(())
    }

    /// start the scanner loop.
    ///
    /// returns:
    /// - broadcast receiver for fish detections
    /// - task handle
    pub async fn run(mut self) -> (broadcast::Receiver<i32>, JoinHandle<()>) {
        info!("monitoring fishing...");

        let (fish_tx, fish_rx) = broadcast::channel::<i32>(1);
        let mut scanning = true;

        let handle = tokio::spawn(async move {
            loop {
                // check for commands (non-blocking)
                while let Ok(cmd) = self.cmd_rx.try_recv() {
                    match cmd {
                        ScannerCommand::ZeroMemory => {
                            if let Err(e) = self.zero_memory() {
                                warn!("failed to zero memory: {:#?}", e);
                            }
                        }
                        ScannerCommand::TogglePause(bool) => {
                            scanning = bool;
                            debug!("scanning {}", if scanning { "resumed" } else { "paused" });
                        }
                    }
                }

                if !scanning {
                    // paused – sleep briefly and re-check.
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }

                match self.poll_fish() {
                    Ok(fish_id) => {
                        if fish_id != 0 && fish_id != self.last_fish_id {
                            self.last_fish_id = fish_id;

                            if let Err(e) = fish_tx.send(fish_id) {
                                error!("broadcast error:\n{:#?}", e);
                            }
                        } else if fish_id == 0 && self.last_fish_id != 0 {
                            debug!("fish reset (was {})", self.last_fish_id);
                            self.last_fish_id = 0;
                        }
                    }
                    Err(e) => {
                        error!("error:\n{:#?}", e);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }

                tokio::time::sleep(Duration::from_millis(self.config.scanner.poll_interval_ms))
                    .await;
            }
        });

        (fish_rx, handle)
    }
}
