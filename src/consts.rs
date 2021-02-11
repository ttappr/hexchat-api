#![allow(dead_code)]
// Channel flags.
const CHAN_FLAG_CONNECTED: i32           = 0x0001;
const CHAN_FLAG_CONNECTING: i32          = 0x0002;
const CHAN_FLAG_MARKED_AWAY: i32         = 0x0004;
const CHAN_FLAG_END_OF_MOTD: i32         = 0x0008;
const CHAN_FLAG_HAS_WHOX: i32            = 0x0010;
const CHAN_FLAG_HAS_IDMSG: i32           = 0x0020;
const CHAN_FLAG_HIDE_JOIN: i32           = 0x0040;
const CHAN_FLAG_HIDE_JOIN_UNSET: i32     = 0x0080;
const CHAN_FLAG_BEEP_ON_MESSAGE: i32     = 0x0100;
const CHAN_FLAG_BLINK_TRAY: i32          = 0x0200;
const CHAN_FLAG_BLINK_TASKBAR: i32       = 0x0400;
const CHAN_FLAG_LOGGING: i32             = 0x0800;
const CHAN_FLAG_LOGGING_UNSET: i32       = 0x1000;
const CHAN_FLAG_SCROLLBACK: i32          = 0x2000;
const CHAN_FLAG_SCROLLBACK_UNSET: i32    = 0x4000;
const CHAN_FLAG_STRIP_COLORS: i32        = 0x8000;
const CHAN_FLAG_STRIP_COLORS_UNSET: i32  =0x10000;    
// Channel types.
const CHAN_TYPE_SERVER: i32                   = 1;
const CHAN_TYPE_CHANNEL: i32                  = 2;
const CHAN_TYPE_DIALOG: i32                   = 3;
const CHAN_TYPE_NOTICE: i32                   = 4;
const CHAN_TYPE_SNOTICE: i32                  = 5;    

// DCC status values.
const DCC_STATUS_QUEUED: i32                  = 0;
const DCC_STATUS_ACTIVE: i32                  = 1;
const DCC_STATUS_FAILED: i32                  = 2;
const DCC_STATUS_DONE: i32                    = 3;
const DCC_STATUS_CONNECTING: i32              = 4;
const DCC_STATUS_ABORTED: i32                 = 5;

// Table online has these listed with values 0, 1, 1, 1 which can't be
// right. So I made them sequential ints.
const DCC_TYPE_SEND: i32                      = 0;
const DCC_TYPE_RECIEVE: i32                   = 1;
const DCC_TYPE_CHATRECV: i32                  = 2;
const DCC_TYPE_CHATSEND: i32                  = 3;

// The table online has these "flags" listed as sequential ints.
// I need to verify whether the online page is wrong, or my understanding
// of what "flags" means wrt HexChat is wrong.
const IGN_FLAG_PRIVATE: i32                = 0x01;
const IGN_FLAG_NOTICE: i32                 = 0x02;
const IGN_FLAG_CHANNEL: i32                = 0x04;
const IGN_FLAG_CTCP: i32                   = 0x08;
const IGN_FLAG_INVITE: i32                 = 0x10;
const IGN_FLAG_UNIGNORE: i32               = 0x20;
const IGN_FLAG_NOSAVE: i32                 = 0x40;
const IGN_FLAG_DCC: i32                    = 0x80;

// IRC color codes. Use these in strings printed to he/xchat.
const IRC_WHITE: &str                  = "\x0300";
const IRC_BLACK: &str                  = "\x0301";
const IRC_NAVY: &str                   = "\x0302";
const IRC_GREEN: &str                  = "\x0303";
const IRC_RED: &str                    = "\x0304";
const IRC_MAROON: &str                 = "\x0305";
const IRC_PURPLE: &str                 = "\x0306";
const IRC_OLIVE: &str                  = "\x0307";
const IRC_YELLOW: &str                 = "\x0308";
const IRC_LIGHT_GREEN: &str            = "\x0309";
const IRC_TEAL: &str                   = "\x0310";
const IRC_CYAN: &str                   = "\x0311";
const IRC_ROYAL_BLUE: &str             = "\x0312";
const IRC_MAGENTA: &str                = "\x0313";
const IRC_GRAY: &str                   = "\x0314";
const IRC_LIGHT_GRAY: &str             = "\x0315";

// IRC text format codes. Use these in strings printed to he/xchat.
const IRC_BOLD: &str                     = "\x02"; //"\002";
const IRC_HIDDEN: &str                   = "\x08"; //"\010";
const IRC_UNDERLINE: &str                = "\x1F"; //"\037";
const IRC_ORIG_ATTRIBS: &str             = "\x0F"; //"\017";
const IRC_REVERSE_COLOR: &str            = "\x16"; //"\026";
const IRC_BEEP: &str                     = "\x07"; //"\007";
const IRC_ITALICS: &str                  = "\x1D"; //"\035";
