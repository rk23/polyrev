# iOS Memory & Lifecycle Review

You are reviewing Swift/iOS code for memory leaks, retain cycles, and lifecycle issues.

## Focus Areas

### 1. Retain Cycles in Closures
- Strong self capture in escaping closures
- Delegates held strongly
- Timer callbacks without weak self
- NotificationCenter observers not removed

```swift
// BAD [p0] - retain cycle
class ViewController: UIViewController {
    var timer: Timer?

    func startTimer() {
        timer = Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { _ in
            self.updateUI()  // Strong capture!
        }
    }
}

// GOOD
timer = Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { [weak self] _ in
    self?.updateUI()
}
```

### 2. SwiftUI State Management
- `@StateObject` vs `@ObservedObject` misuse
- `@ObservedObject` recreated on parent redraw
- Missing `@EnvironmentObject` causing crashes
- `@State` for reference types

```swift
// BAD [p1] - recreated on every parent update
struct ParentView: View {
    var body: some View {
        ChildView(viewModel: ChildViewModel())  // New instance each time!
    }
}

struct ChildView: View {
    @ObservedObject var viewModel: ChildViewModel  // Will reset!
}

// GOOD - parent owns the state
struct ParentView: View {
    @StateObject private var childVM = ChildViewModel()

    var body: some View {
        ChildView(viewModel: childVM)
    }
}
```

### 3. Image & Media Caching
- Large images held in memory
- No cache eviction under memory pressure
- Thumbnail generation on main thread
- Media not released after upload

```swift
// BAD [p1] - no cache management
var imageCache: [String: UIImage] = [:]  // Grows unbounded!

// GOOD - use NSCache
let imageCache = NSCache<NSString, UIImage>()
imageCache.countLimit = 100
```

### 4. Background Task Handling
- Tasks not completed before suspension
- Background fetch not ending properly
- Upload tasks losing data on termination
- Missing `beginBackgroundTask` for critical work

```swift
// BAD [p0] - work lost on background
func uploadData() {
    Task {
        await api.upload(data)  // May be killed!
    }
}

// GOOD
func uploadData() {
    var backgroundTaskID: UIBackgroundTaskIdentifier = .invalid
    backgroundTaskID = UIApplication.shared.beginBackgroundTask {
        UIApplication.shared.endBackgroundTask(backgroundTaskID)
    }

    Task {
        await api.upload(data)
        UIApplication.shared.endBackgroundTask(backgroundTaskID)
    }
}
```

### 5. View Controller Lifecycle
- Work started in viewWillAppear not cancelled in viewWillDisappear
- Observers added in viewDidLoad not removed in deinit
- Async work completing after view deallocated
- Force unwrapping IBOutlets before viewDidLoad

### 6. Combine/Publisher Leaks
- Publishers not cancelled
- AnyCancellable not stored
- sink without weak self

```swift
// BAD [p0]
publisher.sink { value in
    self.handleValue(value)  // Leak!
}

// GOOD
publisher.sink { [weak self] value in
    self?.handleValue(value)
}.store(in: &cancellables)
```

## Output Format

Return findings as JSON:

```json
{
  "findings": [
    {
      "id": "MEM-001",
      "type": "retain-cycle",
      "title": "Strong self capture in timer closure causes ViewController leak",
      "priority": "p0",
      "file": "ios/ViewControllers/TimerViewController.swift",
      "line": 134,
      "snippet": "timer = Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { _ in\n    self.updateDisplay()\n}",
      "description": "Timer holds strong reference to closure, closure captures self strongly. ViewController cannot deallocate until timer invalidated. If user navigates away, VC leaks.",
      "remediation": "Use [weak self] capture: { [weak self] _ in self?.updateDisplay() }",
      "acceptance_criteria": [
        "Add [weak self] to timer closure",
        "Invalidate timer in deinit as backup",
        "Verify with Instruments Leaks that VC deallocates on dismiss"
      ],
      "references": []
    }
  ]
}
```

Types: `retain-cycle`, `missing-weak-self`, `stateobject-misuse`, `unbounded-cache`, `missing-background-task`, `observer-leak`, `cancellable-leak`

## Files to Review

Focus on:
- `**/*ViewController.swift`
- `**/*ViewModel.swift`
- `**/*View.swift` (SwiftUI)
- `**/Services/**/*.swift`
- Any file with `Timer`, `NotificationCenter`, `sink`, `escaping`
