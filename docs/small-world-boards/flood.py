"""Priority flood (max-path image foresting transform): each pixel goes to
the seed it can reach over the lowest highest-cost path."""
import heapq
import numpy as np


def flood(cost, markers):
    h, w = cost.shape
    c = cost.ravel().astype(np.int32)
    lab = markers.ravel().astype(np.int32).copy()
    done = np.zeros(h * w, bool)
    heap = []
    for i in np.flatnonzero(lab):
        heap.append((int(c[i]), 0, int(i)))
    heapq.heapify(heap)
    tick = 1
    while heap:
        lvl, _, i = heapq.heappop(heap)
        if done[i]:
            continue
        done[i] = True
        y, x = divmod(i, w)
        for j in (i - w if y else -1, i + w if y < h - 1 else -1, i - 1 if x else -1, i + 1 if x < w - 1 else -1):
            if j >= 0 and not done[j] and lab[j] == 0:
                lab[j] = lab[i]
                heapq.heappush(heap, (max(lvl, int(c[j])), tick, j))
                tick += 1
            elif j >= 0 and not done[j] and lab[j] != lab[i]:
                pass
    return lab.reshape(h, w)
