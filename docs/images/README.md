# Screenshot sources

The library screenshots show real mod covers in an isolated demo library. Names and authors come from attached CurseForge project metadata. The capture uses demonstration files, without distributing mod packages or changing an installed game.

Mod names and cover artwork belong to their respective creators.

| Mod | Creator | Cover image |
| --- | --- | --- |
| [Aurum - maxis match denim shorts](https://www.curseforge.com/sims4/create-a-sim/aurum-maxis-match-denim-shorts) | AuSims | [Original cover](https://media.forgecdn.net/avatars/1274/361/638829051417443712.png) |
| [Botanica Tattoo](https://www.curseforge.com/sims4/create-a-sim/botanica-tattoo) | moonmoonsim | [Original cover](https://media.forgecdn.net/avatars/678/867/638068403334197764.png) |
| [GoldFish Little Black Dress LBD3](https://www.curseforge.com/sims4/create-a-sim/goldfish-little-black-dress-lbd3) | GoldFishSims | [Original cover](https://media.forgecdn.net/avatars/1112/546/638664841782167451.png) |
| [GoldFish Urban Elegance](https://www.curseforge.com/sims4/create-a-sim/goldfish-urban-elegance) | GoldFishSims | [Original cover](https://media.forgecdn.net/avatars/1773/192/639126370020867232.png) |
| [Aurum - Serana hairstyle](https://www.curseforge.com/sims4/create-a-sim/aurum-serana-hairstyle) | AuSims | [Original cover](https://media.forgecdn.net/avatars/1324/263/638860931844527814.png) |
| [MC Command Center](https://www.curseforge.com/sims4/mods/mc-command-center) | deaderpool_mccc | [Original cover](https://media.forgecdn.net/avatars/593/897/637967028550536035.png) |
| [QICC - Sweater Weather Collection](https://www.curseforge.com/sims4/create-a-sim/qicc-sweater-weather-collection) | Quirky_Introvert | [Original cover](https://media.forgecdn.net/avatars/748/297/638087897421968953.jpeg) |
| [WonderfulWhims](https://www.curseforge.com/sims4/mods/wonderfulwhims) | TURBODRIVER | [Original cover](https://media.forgecdn.net/avatars/602/578/637983317249528036.png) |

`library-light.png`, `library-dark.png`, and `settings.png` are captured at 1440 × 1000 from the native release binary. The README logo is the PlumCrate wordmark with the existing plumbob icon, [`native/assets/logo.svg`](../../native/assets/logo.svg).

To regenerate the screenshots, build the release binary, then run:

```bash
python3 native/capture_readme.py
```

The command downloads the listed public covers without a Referer header. It uses temporary home, game, and library folders, and checks that taking screenshots leaves their contents unchanged. It does not need a CurseForge API key.
