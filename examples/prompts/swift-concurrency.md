# Swift Concurrency Review

You are reviewing Swift/iOS code for concurrency issues, race conditions, and Swift 6 compatibility.

## Focus Areas

### 1. MainActor Isolation
- UI updates from background threads
- Missing `@MainActor` on view models
- Calling MainActor-isolated code from non-isolated contexts
- `DispatchQueue.main.async` vs `await MainActor.run`

```swift
// BAD [p0] - UI update from background
Task {
    let data = await api.fetch()
    self.tableView.reloadData()  // Not on main thread!
}

// GOOD
Task {
    let data = await api.fetch()
    await MainActor.run {
        self.tableView.reloadData()
    }
}

// BETTER - MainActor ViewModel
@MainActor
class MyViewModel: ObservableObject {
    @Published var items: [Item] = []
}
```

### 2. Sendable Compliance
- Closures capturing non-Sendable types across actor boundaries
- Classes that should be Sendable but aren't marked
- Mutable state shared across tasks without synchronization

```swift
// BAD [p1] - non-Sendable capture
class DataManager {  // Not Sendable
    var cache: [String: Data] = [:]
}

let manager = DataManager()
Task.detached {
    manager.cache["key"] = data  // Race condition!
}

// GOOD
actor DataManager {
    var cache: [String: Data] = [:]
}
```

### 3. Data Races
- Shared mutable state without locks/actors
- Dictionary/Array mutations from multiple tasks
- Property access without synchronization

### 4. Async/Await Patterns
- Mixing completion handlers with async/await incorrectly
- Missing `try` on throwing async calls
- Deadlocks from blocking on async work
- Task cancellation not handled

```swift
// BAD [p1] - blocking
func getData() -> Data {
    var result: Data!
    let semaphore = DispatchSemaphore(value: 0)
    Task {
        result = await api.fetch()
        semaphore.signal()
    }
    semaphore.wait()  // Deadlock risk!
    return result
}
```

### 5. Task Management
- Fire-and-forget tasks losing errors
- Missing task cancellation on view disappear
- Unstructured concurrency where structured would work
- Task priority inversion

## Known Issues

Build warnings show concurrency issues in `CameraViews.swift`. Pay special attention to:
- Camera/AVFoundation operations
- Image processing pipelines
- Media upload tasks

## Output Format

Return findings as JSON:

```json
{
  "findings": [
    {
      "id": "CONCURRENCY-001",
      "type": "main-actor-violation",
      "title": "UI update from background task in ChallengeViewModel",
      "priority": "p0",
      "file": "ios/ViewModels/ChallengeViewModel.swift",
      "line": 89,
      "snippet": "Task {\n    let data = await api.fetchChallenge(id)\n    self.challenge = data  // @Published update off main thread\n}",
      "description": "@Published property updated from background task. Will cause UI glitches or crashes. Xcode shows purple runtime warning.",
      "remediation": "Mark ViewModel as @MainActor or wrap update in MainActor.run { }",
      "acceptance_criteria": [
        "Add @MainActor to ChallengeViewModel class",
        "Verify all @Published updates occur on main actor",
        "Run with Thread Sanitizer to confirm no races"
      ],
      "references": ["https://developer.apple.com/documentation/swift/mainactor"]
    }
  ]
}
```

Types: `main-actor-violation`, `data-race`, `sendable-violation`, `deadlock-risk`, `task-leak`, `completion-handler-misuse`

## Files to Review

Focus on:
- `**/*.swift` files with `Task`, `async`, `await`, `actor`
- `**/ViewModels/**/*.swift`
- `**/Services/**/*.swift`
- `**/Camera*/**/*.swift`
- Any file with `DispatchQueue`, `OperationQueue`, completion handlers
