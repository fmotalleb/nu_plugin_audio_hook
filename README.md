# 🎵 nu_plugin_audio_hook  

A [Nushell](https://www.nushell.sh/) plugin for generating and playing sounds. Supports beeping, tone generation, metadata manipulation, and playback for multiple audio formats.  

---

## ✨ Features  

- **`sound beep`** → Play a simple beep sound.  
- **`sound make`** → Generate a noise with a given frequency and duration.  
- **`sound meta`** → Retrieve metadata (duration, artist, album, etc.) from an audio file.  
- **`sound meta set`** → Modify ID3 metadata frames in an audio file. [More about ID3](https://docs.puddletag.net/source/id3.html).  
- **`sound play`** → Play an audio file. By default, supports FLAC, WAV, MP3, and OGG. Use the `all-decoders` feature to enable AAC and MP4 playback.  

---

## 📌 Usage  

### **Generate a simple noise**  

```bash
sound make 1000 200ms
```  

### **Generate a noise sequence**  

```bash
[ 300.0, 500.0, 1000.0, 400.0, 600.0 ] | each { |it| sound make $it 150ms }
```  

### **Play an audio file (first 3 seconds only)**  

```bash
sound play audio.mp3 -d 3sec
```  

### **Retrieve metadata from an audio file**  

```bash
sound meta audio.mp4
```  

Example Output:  

```
╭──────────────┬────────────────────────────╮
│ duration     │ 4min 5sec 551ms 20µs 408ns │
│ artist       │ SINGER                     │
│ title        │ TITLE                      │
│ album        │ ALBUM                      │
│ album_artist │ SINGER                     │
│ track_no     │ 1                          │
│ total_tracks │ 1                          │
╰──────────────┴────────────────────────────╯
```  

### **Modify ID3 metadata (change the artist tag)**  

```bash
sound meta set audio.mp3 -k TPE1 -v new-artist
```  

### **Play an MP3 file using its metadata duration**  

```bash
sound meta audio.mp4 | sound play audio.mp3 -d $in.duration
```  

### **List all available ID3 frame names**  

```bash
sound meta list
```  

---

## 🔧 Installation  

### (If using GNU/Linux) Install ALSA development package (by distro)

**Debian / Ubuntu**

```bash
sudo apt update
sudo apt install -y libasound2-dev pkg-config
```

**RHEL / CentOS / Rocky / Alma**

```bash
sudo dnf install -y alsa-lib-devel pkgconf-pkg-config
```

**Arch Linux**

```bash
sudo pacman -S --needed alsa-lib pkgconf
```

**openSUSE**

```bash
sudo zypper install alsa-lib-devel pkg-config
```


### 🚀 Recommended: Using [nupm](https://github.com/nushell/nupm)  

```bash
git clone https://github.com/FMotalleb/nu_plugin_audio_hook.git  
nupm install --path nu_plugin_audio_hook -f  
```  

### 🛠️ Manual Compilation  

```bash
git clone https://github.com/FMotalleb/nu_plugin_audio_hook.git  
cd nu_plugin_audio_hook  
cargo build -r --features=all-decoders  
plugin add target/release/nu_plugin_audio_hook  
```  

### 📦 Install via Cargo (using git)  

```bash
cargo install --git https://github.com/FMotalleb/nu_plugin_audio_hook.git --features=all-decoders  
plugin add ~/.cargo/bin/nu_plugin_audio_hook  
```  

### 📦 Install via Cargo (crates.io) _Not Recommended_  
>
> _Since I live in Iran and crates.io often restricts package updates, the version there might be outdated._  

```bash
cargo install nu_plugin_audio_hook --features=all-decoders  
plugin add ~/.cargo/bin/nu_plugin_audio_hook  
```  

---

## 🔍 Supported Features  

You can enable specific features when compiling or installing:  

- **`full`** → Enables all features below.  
- **`flac`** (default) → FLAC format support.  
- **`vorbis`** (default) → OGG Vorbis support.  
- **`wav`** (default) → WAV format support.  
- **`minimp3`** → MP3 decoding.  
- **`symphonia-all`** → Enables all Symphonia-based decoders:  
  - `symphonia-aac` → AAC decoding.  
  - `symphonia-flac` → FLAC decoding.  
  - `symphonia-isomp4` → MP4 (audio) decoding.  
  - `symphonia-mp3` (default) → MP3 decoding.  
  - `symphonia-vorbis` → OGG Vorbis decoding.  
  - `symphonia-wav` → WAV decoding.  
