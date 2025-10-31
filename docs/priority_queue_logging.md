# Priority Queue Scheduler Logging Guide

## Overview
We've added comprehensive logging to the priority queue scheduler to verify it's working correctly. The logging uses distinct emojis and prefixes to make it easy to track different components.

## Logging Categories

### 🔵 Priority Queue Operations (`[PRIORITY QUEUE]`)
- **Adding tasks**: Shows task details and queue size changes
- **Peeking**: Shows next task without removing it
- **Popping**: Shows task removal and queue size changes  
- **Removing**: Shows task removal by ID

### 🚀 Scheduler Main Loop (`[SCHEDULER]`)
- **Startup**: Confirms scheduler initialization
- **Iterations**: Shows each scheduler check cycle
- **Sleep timing**: Shows calculated sleep duration until next task
- **Due tasks**: Shows when tasks become ready for notification
- **Error handling**: Shows retry behavior on failures

### 📬 Task Processing (`[TASK PROCESSING]`)
- **Task details**: Shows task info when processing notifications
- **Queue operations**: Confirms task removal from queue
- **Notifications**: Shows notification sending status
- **Post-processing**: Shows single task deletion vs recurring task rescheduling
- **Retry logic**: Shows failed notification retry scheduling

### 🔄 Orchestrator Operations (`[ORCHESTRATOR]`)
- **Task creation**: Shows new tasks being scheduled
- **Task editing**: Shows old task removal and new task scheduling
- **Post-notification**: Shows single task cleanup vs recurring task rescheduling
- **Startup**: Shows existing tasks being loaded into scheduler

## Key Behaviors to Verify

### ✅ Efficient Priority Queue Behavior
1. **O(1) Peek Operations**: Only checks the next due task, not all tasks
2. **Precise Sleep Timing**: Sleeps exactly until next task is due (no fixed 60s intervals)
3. **Queue Size Tracking**: Monitor queue size changes to ensure proper additions/removals

### ✅ Single Task Lifecycle
1. Task created → Added to queue
2. Task becomes due → Removed from queue → Notification sent
3. Post-notification → Removed from repository and scheduler
4. **Expected logs**: Creation, due processing, removal

### ✅ Weekly Task Lifecycle  
1. Task created → Added to queue
2. Task becomes due → Removed from queue → Notification sent
3. Post-notification → Next occurrence calculated → Re-added to queue
4. **Expected logs**: Creation, due processing, rescheduling

### ✅ Task Editing Behavior
1. Edit initiated → Old task removed from scheduler
2. New version → Added to scheduler with updated time
3. **Expected logs**: Removal, re-addition with new schedule

## Sample Log Output Patterns

### Startup Sequence:
```
🔄 [ORCHESTRATOR] Loading existing tasks into priority queue scheduler...
📅 [ORCHESTRATOR] Loaded existing task #1 'Weekly Meeting' scheduled for 2025-11-01 14:00:00 UTC
🔵 [PRIORITY QUEUE] Added task #1 'Weekly Meeting' scheduled for 2025-11-01 14:00:00 UTC | Queue size: 0 → 1
✅ [ORCHESTRATOR] Initialized scheduler with 1 existing tasks
🚀 [SCHEDULER] Priority Queue Scheduler started!
```

### Task Due Processing:
```
🔄 [SCHEDULER] Iteration started at 2025-11-01 14:00:05 UTC
👁️  [PRIORITY QUEUE] Peeking next task: #1 'Weekly Meeting' due at 2025-11-01 14:00:00 UTC | Queue size: 1
⚡ [SCHEDULER] Task #1 'Weekly Meeting' is DUE NOW! Processing notification...
📬 [TASK PROCESSING] Processing due task #1 'Weekly Meeting' (User: 123456, Recurring: true)
⬆️  [PRIORITY QUEUE] Popped task: #1 'Weekly Meeting' | Queue size: 1 → 0
📤 [TASK PROCESSING] Sending notification for task #1 'Weekly Meeting'...
✅ [TASK PROCESSING] Notification sent successfully for task #1
🔄 [TASK PROCESSING] Handling post-notification processing for task #1...
🔄 [ORCHESTRATOR] Post-notification handling for task #1 'Weekly Meeting' (Type: Recurring)
📅 [ORCHESTRATOR] Next occurrence calculated: 2025-11-08 14:00:00 UTC for task #1
🔵 [PRIORITY QUEUE] Added task #1 'Weekly Meeting' scheduled for 2025-11-08 14:00:00 UTC | Queue size: 0 → 1
```

### Sleep Behavior:
```
👁️  [PRIORITY QUEUE] Peeking next task: #1 'Weekly Meeting' due at 2025-11-08 14:00:00 UTC | Queue size: 1
⏰ [SCHEDULER] Next task #1 'Weekly Meeting' due in 7d 0h 0m 0s. Sleeping until 2025-11-08 14:00:00 UTC
```

## Performance Verification

The logs should show:
- **No polling behavior**: No regular 60-second checks when no tasks are due
- **Exact timing**: Sleep durations match time until next task
- **Efficient operations**: Only peek operations until tasks are due
- **Queue size consistency**: Sizes should always be accurate

## Testing Recommendations

1. **Create tasks with different due times** to see sleep duration calculations
2. **Create both single and recurring tasks** to verify different post-notification behavior  
3. **Edit tasks** to see scheduler updates
4. **Let tasks become due** to see the complete notification and cleanup/rescheduling process
5. **Start bot with existing tasks** to see startup loading behavior

The comprehensive logging will help you verify that the priority queue scheduler is working efficiently and correctly handling all edge cases.