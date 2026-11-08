import time
from collections import deque


class Call:
    def __init__(self, user1, channel1, guild1, user2, channel2, guild2):
        self.user1 = str(user1)
        self.channel1 = str(channel1)
        self.guild1 = str(guild1) if guild1 is not None else ""
        self.user2 = str(user2)
        self.channel2 = str(channel2)
        self.guild2 = str(guild2) if guild2 is not None else ""
        self.last_message_at = time.monotonic()

    def partner_destination(self, user_id):
        user_id = str(user_id)
        if self.user1 == user_id:
            return (self.channel2, self.guild2)
        if self.user2 == user_id:
            return (self.channel1, self.guild1)
        return None

    def contains(self, user_id):
        user_id = str(user_id)
        return self.user1 == user_id or self.user2 == user_id


class CallManager:
    def __init__(self):
        self.user_queue = deque()
        self.active_calls = []


def pair_waiting_users(manager, max_active_pairs):
    new_pairs = []
    while len(manager.user_queue) >= 2 and len(manager.active_calls) < max_active_pairs:
        user1, channel1, guild1, _ = manager.user_queue.popleft()
        user2, channel2, guild2, _ = manager.user_queue.popleft()
        call = Call(user1, channel1, guild1, user2, channel2, guild2)
        manager.active_calls.append(call)
        new_pairs.append(call)
    return new_pairs
