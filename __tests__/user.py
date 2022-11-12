#!/usr/bin/env python

import asyncio
import websockets

async def hello():
    # let = "63307b31d0c18856548cef9d"
    async with websockets.connect("ws://localhost:7878/pair/63613b6e50ddc3b5ef1cca7c/63307b31d0c18856548cef9d") as websocket:
        # await websocket.send("Hello world!")
        while True:
            message = await websocket.recv()
            print(message)

asyncio.run(hello())