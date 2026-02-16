# Features Implementable in libobs-rs (Excluding Plugins, macOS, Scripting, Browser, Virtual Camera, Frontend)

## Core Output Types
- **RTMP Streaming Output** - Live streaming to RTMP-compatible services (Twitch, YouTube, Facebook Live)
- **WHIP/WebRTC Output** - Modern low-latency streaming protocol
- **HLS Muxer** - HTTP Live Streaming with adaptive bitrate support
- **MPEG-TS Muxer** - MPEG Transport Stream for streaming protocols
- **Fragmented MP4/MOV** - Advanced MP4 format with recovery options
- **File Output Muxing** - Generic FFmpeg-based file muxing

## Windows-Specific Input Sources
- **Enhanced Game Capture** - Improved exclusive fullscreen detection
- **Duplicator Monitor Capture** - DirectX-based DXGI display duplication
- **Advanced Window Capture** - Improved window targeting and cursor handling

## Audio Features
- **Fader System** - Multiple fader types (Cubic, IEC, Logarithmic) with dB mapping
- **Volume Meter/Volmeter** - Peak and RMS level monitoring
- **Audio Monitor** - Per-source audio monitoring with device selection
- **Multi-Channel Audio** - Surround sound mixing (5.1, 7.1)
- **Audio Ducking** - Windows audio ducking support
- **Balance Types** - Sine law, square law, and linear panning

## Advanced Encoder Support
- **OpenH264 Encoder** - Cisco's H.264 codec integration
- **SVT-AV1 Encoder** - Intel Scalable Video Technology AV1
- **AOM-AV1 Encoder** - Alliance for Open Media AV1
- **FDK-AAC Encoder** - Fraunhofer's AAC implementation
- **PCM Variants** - 16-bit, 24-bit, 32-bit PCM encoders
- **ALAC Encoder** - Apple Lossless Audio Codec
- **FLAC Encoder** - Free Lossless Audio Codec
- **Encoder Parameter Expansion** - More granular control over existing encoder settings

## Service Integration
- **Streaming Service Registry** - Dynamic service configuration system
- **Twitch Integration** - Channel info lookup and auto-configuration
- **YouTube Integration** - YouTube Live configuration
- **Amazon IVS** - Amazon Interactive Video Service support
- **Custom RTMP Services** - Generic RTMP service templates
- **Service Ingest Discovery** - Automatic ingest URL optimization
- **DaCast, NimoTV, Showroom** - Additional platform support

## Profile & Scene Collection Management
- **Enhanced Profile System** - Creation, duplication, deletion with proper isolation
- **Profile-Specific Settings** - Per-profile encoder/audio/video configuration
- **Scene Collection Persistence** - Robust JSON-based storage with versioning
- **Import/Export** - Scene collection backup and sharing
- **Coordinate System Migration** - Handle absolute vs relative coordinate conversion
- **Module Data Storage** - Custom module data within collections
- **Collection-Specific Transitions** - Per-collection transition templates
- **Quick Transitions** - Pre-configured transition templates

## Hotkey System
- **Global Hotkey Registration** - Complete hotkey binding system
- **Key Combination Support** - Multi-modifier hotkey support
- **Hotkey Context** - Conditional hotkey activation
- **Key Code Mapping** - Support for 200+ keyboard keys
- **Mouse Button Hotkeys** - Support for up to 12 mouse buttons

## Source Interaction & Input
- **Mouse Event Handling** - Position tracking, button states
- **Keyboard Event Processing** - Key codes and modifiers
- **Touch/Pointer Support** - Advanced input device support
- **Modifier Key Tracking** - Ctrl, Shift, Alt, Cmd state tracking

## Advanced Source Features
- **Source Grouping** - Hierarchical source organization
- **Nested Scenes** - Scenes within scenes with transform inheritance
- **Show/Hide Transitions** - Per-item visibility transition effects
- **Source Transformation** - Advanced scaling algorithms, blending modes, crop/position
- **Deinterlacing** - Video deinterlace filter integration
- **Transform Inheritance** - Group-based transform propagation

## Property System Enhancements
- **Advanced Property Types** - Nested property groups, dynamic updates
- **Enum Properties** - Dropdown selections with constraints
- **Button Properties** - Clickable action properties
- **Path Properties** - File/directory selection with validation
- **Color Properties** - Color picker integration
- **Conditional Properties** - Show/hide based on other property values
- **Property Callbacks** - Real-time property change notifications

## Recording & Streaming State
- **Advanced State Management** - Detailed start/stop/pause/resume control
- **Stream State Queries** - Active streaming status and metadata
- **Recording State Queries** - Active recording status and file info
- **Output Delay** - Stream delay/latency control
- **Bitrate Monitoring** - Real-time bitrate tracking
- **Dropped Frame Tracking** - Network/CPU performance metrics

## Missing Files & Recovery
- **Missing File Detection** - Validate source file existence
- **Missing File Recovery** - Locate and restore missing sources
- **Backup Mechanism** - Automatic scene/profile backups

## Audio Track Selection
- **Multi-Track Output** - Multiple audio tracks in output
- **Track Mixing** - Per-track volume and EQ
- **Track Mapping** - Flexible audio source to track mapping