### Priority Queue Scheduler

**File:** `src/infrastructure/scheduler/priority_queue_scheduler.rs`  
**Usage:** Started in `bot.rs` during Discord client initialization using `TaskOrchestrator` coordination

##### Description

The scheduler is an **efficient asynchronous system** using a priority queue that:

- **Only checks the next due task** (O(1) operation) instead of scanning all tasks
- **Sleeps precisely** until the next notification time - no fixed intervals
- **Maintains tasks ordered by time** using BinaryHeap with earliest tasks having highest priority
- **Processes notifications immediately** when tasks become due
- **Handles both single and recurring tasks** through TaskOrchestrator coordination
- **Automatically reschedules weekly tasks** for their next occurrence
- **Sends Discord notifications** via DM, channel, or both based on user preferences

##### Technical Implementation

**Priority Queue Structure:**
- Uses `BinaryHeap<Reverse<ScheduledTask>>` for min-heap behavior (earliest first)
- `ScheduledTask` entities contain minimal data for memory efficiency
- Thread-safe access through Tokio `Mutex` for concurrent operations

**Scheduler Loop Behavior:**
1. `peek_next_task()` - Check the earliest scheduled task (O(1))
2. If task is due: process immediately and `pop()` from queue
3. If not due: sleep exactly until that task's scheduled time
4. If no tasks: sleep for 5 minutes and recheck

**Complexity Analysis:**
- **Task Checking**: O(1) - only peeks at next task regardless of total count
- **Task Addition**: O(log n) - maintains heap ordering when adding new tasks  
- **Task Removal**: O(log n) - efficient removal while preserving structure
- **Memory Usage**: Minimal - only essential task data in queue

##### Purpose

- **Maximum Efficiency**: Scales to thousands of tasks without performance degradation
- **Precise Timing**: Notifications delivered at exact scheduled moment (no ±60s windows)
- **Resource Optimization**: Eliminates continuous database polling and unnecessary checks  
- **Production Ready**: Built for multi-server Discord bot hosting with optimal performance
- **Future-Proof Architecture**: Clean separation allows easy database migration from JSON storage

### Geo-Mapping service

**File:** `src\application\services\geo_mapping_service.rs`
Usage: Used by TimezoneService to resolve geographic queries into valid timezone identifiers

##### Description

The Geo-Mapping Service provides a lightweight, in-memory geographic lookup system that maps countries, U.S. states, and Canadian provinces to their corresponding IANA timezone identifiers.
Exposes several lookup functions and a unified search method that:

- Converts geographic names (country, state, province) into timezone strings
- Handles case-insensitive input
- Searches across multiple predefined mappings
- Acts as a fallback mechanism for more precise or fuzzy timezone searches performed by the TimezoneManager
- All UTC timezones here: `src\infrastructure\data\timezones.json`

##### Purpose

Provides fast, static geographic-to-timezone resolution without requiring a database
Simplifies timezone inference for user input such as "Brazil" or "California"
Enables the TimezoneService to perform combined searches using both static mappings and fuzzy timezone data
Serves as a foundation for a future database-backed GeoMappingRepository, allowing migration to MySQL or other persistent storage solutions

### Tasks Behavior

Individual tasks: DELETED after notification
Weekly Tasks (recurring tasks) are automatically RESCHEDULED after their time arrives (they are not deleted)

There's no "completed" status - Implied completeness by deletion/reprogramming

### Core Design Principles

#### Architectural Consistency

Despite different implementation approaches, the commands maintain architectural consistency:

- **Shared Foundation**: All commands delegate business logic to `TaskService`
- **Timezone Integration**: Unified timezone handling through `TimezoneService`
- **Domain-Driven Design**: Common domain entities (`Task`, `Recurrence`, `WeekdayFormat`)
- **Error Handling**: Consistent user feedback patterns and error management

#### Hexagonal Architecture

The bot follows basic clean architecture principles with clear boundaries:

- **Commands**: User interaction handlers (Discord adapters)
- **Services**: Business logic orchestration (application layer)
- **Domain**: Core business entities and rules (heart of the system)
- **Infrastructure**: External concerns like storage, timezone data, Discord API

#### Timezone-Aware Scheduling

- All task times are stored in UTC
- User-facing displays show local times based on configured timezones
- Geographic mapping enables intuitive timezone setup
- Automatic conversion ensures correct scheduling across timezones

#### Extensibility Framework

The architecture supports natural growth:

- New task types can be added through domain extensions
- Additional notification methods integrate seamlessly
- Geographic and timezone data can evolve without breaking changes
- Command structure allows easy addition of new user interactions
