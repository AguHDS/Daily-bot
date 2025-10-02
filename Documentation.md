### Scheduler

**File:** `src/application/scheduler/scheduler_tokio.rs`  
**Usage:** Called in `handlers.rs` inside the `ready` event of the `CommandHandler`

##### Description
The scheduler is an **asynchronous loop** that:

- Periodically checks the task repository
- Finds tasks whose **scheduled time has passed** and are **not yet completed**
- Prints a reminder message in the console for each task
- Marks tasks as completed to avoid repeated reminders

##### Purpose
- Automates the execution of scheduled tasks
- Simulates **automatic reminders** for users
- Can be extended in the future to **send messages directly in Discord** instead of just printing to the console
