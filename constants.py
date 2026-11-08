# ALOHA constants (migrated from Rust)
COMMAND_PREFIX = "u!"  # the only accepted command prefix; bare `!cmd` is not a command
MAX_ACTIVE_PAIRS = 100
MAX_QUEUE_SIZE = 200
MIN_SEND_INTERVAL = 0.5  # seconds
QUEUE_TIMEOUT = 120  # seconds
CALL_IDLE_TIMEOUT = 300  # seconds
SWEEP_INTERVAL = 15  # seconds
STATUS_ROTATE_INTERVAL = 4 * 60 + 20  # seconds (4 minutes 20 seconds)
STATUSES = [
    "u!userphone - tell your friends about me",
    "u!userphone - meet someone new",
    "u!userphone - friendly, kind chats",
    "u!userphone - say hi, be kind",
    "u!hangup - when you're done chatting",
    "u!help - for commands",
]
RELAY_CONTENT_LIMIT = 2000
INVISIBLE = '\u200b'

COLOR_CONNECTED = 0x2ECC71
COLOR_DISCONNECTED = 0xE74C3C
COLOR_INFO = 0x27D3F5

LINK_MARKERS = ["http://", "https://", "www.", "data:"]
