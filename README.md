# terraria-autofish

automated fishing for terraria because i have the attention span of a goldfish and i hate fishing mechanics (ESPECIALLY IN TERRARIA).

## what is this?
it's technically a cheat that operates with full read **and write** permissions on the game process. while it doesn't inject a dll (it runs as a separate executable), it manipulates memory directly to "zero out" stale data, making it functionally invasive. THIS IS FOR EDUCATIONAL PURPOSES ONLY. use at your own risk.

it's written in rust because i like pain and performance. it uses pattern scanning to find the jit-compiled assembly code in memory so it works across game restarts without needing pointer chains (which break constantly because of aslr and .net's fragmented heap).

## read the blog
want to read the full story of how i built this? check out the blog post:
[blog.guswid.com/terraria-autofish](https://blog.guswid.com/terraria-autofish)

## features
- **memory-based detection**: no ocr, no screen reading. it reads the fish id directly from ram. latency is in microseconds.
- **whitelist system**: only catch what you want. filter by name (e.g., "crate", "honeyfin").
- **auto-potions**: drinks fishing/sonar/crate potions for you on a timer.
- **background processing**: works even if the game isn't focused (though you need to be in the game to reel in, obviously).
- **fast**: uses `rayon` for parallel memory scanning. it finds the hook in milliseconds.

## how to use
1. **download/build**:
   you need rust installed.
   ```bash
   git clone https://github.com/r3dlust/terraria-autofish
   cd terraria-autofish
   cargo run --release
   ```
   (or just grab a binary if i ever upload one)

2. **configure**:
   create a `config.toml` file. here is the full default configuration with explanations:
   ```toml
   [fisher]
   rod_slot = "Num1"
   fishes = ["honeyfin"] # fish names, case insensitive, pattern match (searches for given strings in fish name)

   [fisher.potions]
   # sonar potion slot, remove key to disable sonar potion usage. the macro does not need the sonar potion to work and can detect fishes without it
   # for those who feel like telepathically knowing what fish you'll hook is cheating...
   sonar_potion = "Num2"
   # each potion can have their respective duration time overriden,
   # this is mostly in case you use the alchemy flask's "Alchemic Enhancement" buff, which increases potion duration by 20%
   # sonar_potion_duration_secs = 480 # 576 with alchemy flask buff

   # having a potion's bind commented out will disable the macro"s usage of that potion
   # crate_potion = "Num3"
   # crate_potion_duration_secs = 240 # 288 with alchemy flask buff

   # fishing_potion = "Num4"
   # fishing_potion_duration_secs = 480 # 576 with alchemy flask buff

   # food = "Num5" # food slot, remove key to disable food usage. useful in the `constant` seed (or worlds with the hunger system) to avoid dying of starvation.

   # obtained by doing (<FOOD_BUFF_DURATION> min + 8 + 8) * 60 (on the getfixedboi seed)
   # or by doing (<FOOD_BUFF_DURATION> min + 5 + 5) * 60 (on the constant seed)
   # these extra 16/10 minutes are the leeway the game gives before hunger starts dealing damage. (8/5 minutes peckish, 8/5 minutes hungry)
   food_duration_secs = 1440

   # optional settings for overriding hotkeys, this section is not required at all
   # [fisher.hotkeys]
   # toggle = "F6" # default is "BackSlash"
   # pause = "F7" # default is "RightBracket"

   # optional settings for the scanner, this section is not required at all.
   # do not change these settings unless you know what you're doing,
   # as they can cause the scanner to not work properly (or eat up more resources than necessary)
   # [scanner]
   # poll_interval_ms = 10 # default is 25
   ```

3. **run**:
   start terraria, enter a world, cast your line.
   run the bot. it'll scan memory, find the fishing context, and tell you it's ready.
   press `\` (backslash) to toggle it on. `]` (right bracket) to pause.

## technical jargon
for the nerds:
- **memory scanning**: finds the `Terraria.Projectile.FishingCheck` method signature in the jit heap using a custom pattern scanner.
- **jit hooking & pointer arithmetic**: extracts the static `_context` field address from the assembly dynamically at runtime. from there, we dereference the `FishingContext`, add a fixed offset to find the internal `fisher` struct (of type `FishingAttempt`), and finally land on the `rolledItemDrop` integer.
- **the duplicate fish fix**: since we're polling memory (not hooking a function call), catching the same fish ID twice in a row looks identical to the scanner (the integer value doesn't change). to fix this, we utilize our write permissions to **zero out** the memory address of `rolledItemDrop` immediately after a successful catch. this resets the state so the next detection is guaranteed to be a fresh event. safe to do because the game recalculates this value on every bite frame anyway.
- **performance**: profiled with `samply`. scanning is parallelized with `rayon` to keep latency in the microseconds range.
- **stack**: rust, `windows-rs` for process handles, `rdev` for input simulation.

### under the hood
this is the assembly pattern we scan for in the jit heap. it's from the `Terraria.Projectile.FishingCheck()` method. we find where the `FishingContext` is loaded into a register, extract that static address, and follow it.

```rust
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
```

## credits
me. 
dnspy (for letting me disassemble the .net bytecode and find the offsets).
and the weird font terraria uses for making ocr impossible.

## license
mit. see [LICENSE](LICENSE).
