import asyncio
import time


class Pacer:
    def __init__(self, min_send_interval):
        self.min_send_interval = min_send_interval
        self.last_sent = {}
        self.lock = asyncio.Lock()

    async def wait(self, channel_id):
        channel_id = str(channel_id)
        while True:
            wait_time = None
            async with self.lock:
                last = self.last_sent.get(channel_id)
                if last is None:
                    self.last_sent[channel_id] = time.monotonic()
                    wait_time = None
                else:
                    elapsed = time.monotonic() - last
                    if elapsed < self.min_send_interval:
                        wait_time = self.min_send_interval - elapsed
                    else:
                        self.last_sent[channel_id] = time.monotonic()
                        wait_time = None
            if wait_time is None:
                return
            await asyncio.sleep(wait_time)
