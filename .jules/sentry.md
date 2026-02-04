## 2024-05-22 - [RingBuffer Live Lock on Overwrite]
**Learning:** `RingBuffer::try_read` assumed strict consistency and looped infinitely when `write_pos` lapped `read_pos` because sequence numbers never matched expectation. Lossy buffers must explicitly check for "lapping" and fast-forward the reader.
**Action:** When testing lock-free ring buffers, always include a "wrap around" test case where the writer overwrites unread data to verify reader recovery.
