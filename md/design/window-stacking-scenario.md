# Window Stacking Scenario Walkthrough

This document traces through a typical window stacking scenario, detailing the expected behavior and state changes at each step.

## Initial Setup
- **Inset Percentage**: 10% (0.10)
- **Minimum Inset**: 10px
- **Maximum Inset**: 150px

## Scenario Steps

### Step 1: Add Window 1 (Becomes Leader)

**Action**: User adds first Safari window to stack

**Initial State**:
```
Window 1 (Safari #115809):
  - Original Position: (673, 190)
  - Original Size: 574 × 614
  - Role: Not in stack
```

**Operations**:
1. Store original frame: `(673, 190, 574, 614)`
2. Mark as leader: `isLeader = true`
3. Set as current leader window
4. Setup drag detection for this window
5. Focus window (bring to front)

**Final State**:
```
Window 1 (Safari #115809):
  - Position: (673, 190) [unchanged]
  - Size: 574 × 614 [unchanged]
  - Role: LEADER
  - Stored Original Frame: (673, 190, 574, 614)
  - Z-Order: Front
```

**Stack**: `[Window 1 (LEADER)]`

---

### Step 2: Add Window 2 (Becomes Follower)

**Action**: User adds second Safari window to stack

**Initial State**:
```
Window 1 (Safari #115809):
  - Position: (673, 190)
  - Size: 574 × 614
  - Role: LEADER

Window 2 (Safari #115807):
  - Original Position: (729, 251)
  - Original Size: 574 × 491
  - Role: Not in stack
```

**Calculations**:
```
Leader Frame: (673, 190, 574, 614)

Horizontal Inset = max(10, min(150, 574 × 0.10)) = 57.4px
Vertical Inset = max(10, min(150, 614 × 0.10)) = 61.4px

Follower Frame:
  - X: 673 + 57.4 = 730.4
  - Y: 190 + 61.4 = 251.4
  - Width: 574 - (2 × 57.4) = 459.2
  - Height: 614 - (2 × 61.4) = 491.2
```

**Operations**:
1. Store Window 2's original frame: `(729, 251, 574, 491)`
2. Calculate follower frame based on current leader
3. Move Window 2 to follower position: `(730.4, 251.4, 459.2, 491.2)`
4. Mark as follower: `isLeader = false`
5. **Send Window 2 to back** (behind leader)
6. Do NOT switch leadership
7. Do NOT focus Window 2

**Final State**:
```
Window 1 (Safari #115809):
  - Position: (673, 190) [unchanged]
  - Size: 574 × 614 [unchanged]
  - Role: LEADER
  - Z-Order: Front

Window 2 (Safari #115807):
  - Position: (730.4, 251.4)
  - Size: 459.2 × 491.2
  - Role: FOLLOWER
  - Stored Original Frame: (729, 251, 574, 491)
  - Z-Order: Behind Window 1
```

**Stack**: `[Window 1 (LEADER), Window 2 (FOLLOWER)]`

---

### Step 3: User Drags Window 1

**Action**: User clicks and drags Window 1 by 50px right, 30px down

**Initial State**:
```
Window 1: Position (673, 190), Size (574, 614), LEADER
Window 2: Position (730.4, 251.4), Size (459.2, 491.2), FOLLOWER
```

**Drag Detection**:
1. CGEvent tap detects mouse down on Window 1
2. Verify it's our leader window (ID matches)
3. Start PositionTracker with 20ms polling

**During Drag** (each 20ms poll):
```
Poll 1: Window 1 at (673, 190) → no change
Poll 2: Window 1 at (680, 193) → delta (+7, +3)
  - Move Window 2 to (737.4, 254.4)
Poll 3: Window 1 at (695, 201) → delta (+15, +8)
  - Move Window 2 to (745.4, 259.4)
...
Final: Window 1 at (723, 220) → delta (+28, +19)
  - Move Window 2 to (758.4, 270.4)
```

**On Mouse Up**:
1. Stop PositionTracker
2. Update stored positions

**Final State**:
```
Window 1 (Safari #115809):
  - Position: (723, 220) [moved +50, +30]
  - Size: 574 × 614 [unchanged]
  - Role: LEADER

Window 2 (Safari #115807):
  - Position: (780.4, 281.4) [moved +50, +30]
  - Size: 459.2 × 491.2 [unchanged]
  - Role: FOLLOWER
```

---

### Step 4: User Clicks "Next" (Window 2 Becomes Leader)

**Action**: User clicks Next button to switch leadership

**Initial State**:
```
Window 1: Position (723, 220), Size (574, 614), LEADER
Window 2: Position (780.4, 281.4), Size (459.2, 491.2), FOLLOWER
```

**Operations**:

1. **Update leader flags**:
   - Window 1: `isLeader = false`
   - Window 2: `isLeader = true`

2. **Resize Window 1 to follower size**:
   ```
   Current Leader Frame: (723, 220, 574, 614)
   
   Horizontal Inset = 57.4px
   Vertical Inset = 61.4px
   
   New Window 1 Frame:
     - X: 723 + 57.4 = 780.4
     - Y: 220 + 61.4 = 281.4
     - Width: 574 - 114.8 = 459.2
     - Height: 614 - 122.8 = 491.2
   ```
   
3. **Restore Window 2 to its original size**:
   ```
   Use stored original frame: (729, 251, 574, 491)
   
   But maintain current position relationship:
   - Window 2 was at (780.4, 281.4) as follower
   - Needs to expand back to original size (574 × 491)
   - New position: (723, 220) [same as old leader position]
   ```

4. **Apply changes**:
   - Move/resize Window 1 to `(780.4, 281.4, 459.2, 491.2)`
   - Move/resize Window 2 to `(723, 220, 574, 491)`
   - **Focus Window 2** (brings to front)
   - **Send Window 1 to back**

5. **Update drag detection**:
   - Stop tracking Window 1
   - Start tracking Window 2

**Final State**:
```
Window 1 (Safari #115809):
  - Position: (780.4, 281.4)
  - Size: 459.2 × 491.2
  - Role: FOLLOWER
  - Z-Order: Back

Window 2 (Safari #115807):
  - Position: (723, 220)
  - Size: 574 × 491 [original width, original height]
  - Role: LEADER
  - Z-Order: Front
```

---

## Key Design Decisions

### Why Followers Are Smaller
- **Visual clarity**: Even with lag, followers won't peek out
- **Click protection**: User can't accidentally click on a follower
- **Depth perception**: Smaller size reinforces the stacking metaphor

### Position Management Strategy
- **Store original frames**: Each window remembers its original size
- **Leader uses original size**: When becoming leader, restore original dimensions
- **Followers use calculated size**: Based on current leader's frame

### Z-Order Management
- **Leader always on top**: Use focus/raise actions
- **Followers always behind**: Explicitly send to back
- **Order within followers**: Doesn't matter as long as all are behind leader

### Performance Considerations
- **Drag detection**: Only poll during active drags (not constantly)
- **Batch updates**: Move all followers in one operation if possible
- **Minimize resizing**: Only resize when changing roles, not during drags

## Current Implementation Issues

1. **Auto-leadership on add**: Currently makes new windows leaders immediately
2. **Complex frame calculations**: `calculateLeaderFrame` tries to reverse the follower calculation
3. **Missing z-order management**: No explicit "send to back" for followers
4. **No stored original frames**: Can't properly restore original window sizes

## Proposed Fixes

1. **Remove auto-leadership**: Keep new windows as followers
2. **Store original dimensions**: Add `originalSize` field to WindowInfo
3. **Add z-order methods**: Implement proper window layering
4. **Simplify calculations**: Use stored frames instead of complex reversals