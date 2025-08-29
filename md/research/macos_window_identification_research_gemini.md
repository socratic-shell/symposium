

# **Reliable Window Identification Strategies for macOS Window Management**

## **Executive Summary**

This report provides a comprehensive analysis of the architectural and programmatic challenges associated with reliable window identification on macOS for third-party window management applications. The central issue is a fundamental disconnect between the two primary frameworks for managing window data: Core Graphics (CG) and the Accessibility (AX) API. The CG API offers a low-level, stable identifier for a window's physical representation (CGWindowID), but it is read-only. Conversely, the AX API provides the necessary high-level functionality for window manipulation, but its primary linking attribute, kAXWindowIDAttribute, is non-existent or unreliable for many major applications, including Google Chrome.  
Based on an exhaustive review of public and private APIs, as well as a comparative analysis of leading open-source window managers, this report concludes that a single, universally reliable identifier does not exist. The most effective and robust solution is a **Tiered Window Fingerprint Strategy**. This hybrid model correlates data from both the CG and AX APIs using a common denominator, the Process Identifier (PID), and falls back on a hierarchy of increasingly heuristic attributes. This approach avoids the security compromises of private APIs or disabling System Integrity Protection (SIP), offering a durable and dependable solution for the development of a modern macOS window stacking application. The final sections of this report provide a detailed technical implementation guide and a compatibility matrix for this recommended strategy.

## **1\. The macOS Windowing Paradigm: A Foundation of Dichotomy**

The architecture of macOS presents a unique challenge for third-party applications seeking to programmatically manage windows. Unlike other operating systems that may offer a single, unified window handle, Apple's design separates the visual, low-level representation of a window from its semantic, high-level user interface hierarchy. This architectural separation is a critical design choice that underpins the security and stability of the operating system.

### **1.1. Disentangling Core Graphics and the Accessibility API**

At the lowest level, macOS manages windows as graphical objects via the **Core Graphics** framework. Functions from the CGWindowList family, such as CGWindowListCopyWindowInfo, provide a snapshot of all windows in the current user session.1 The information returned is a CFArray of dictionaries, with each dictionary representing a window and containing key-value pairs for properties like  
kCGWindowBounds, kCGWindowOwnerName, and kCGWindowOwnerPID.2 The  
kCGWindowNumber within this dictionary is the CGWindowID, a 32-bit integer that serves as a stable and persistent identifier for a window's physical existence throughout its lifecycle.3 This API is exceptionally fast and reliable for enumerating windows and retrieving their visual properties, but it is fundamentally a read-only interface; it offers no functions for moving, resizing, or otherwise interacting with a window programmatically.4  
For interactive control, developers must turn to the **Accessibility API**, a framework originally designed to empower assistive technologies like VoiceOver.5 The Accessibility API represents UI elements as a hierarchical tree, with each element being an  
AXUIElementRef.7 A window is a type of  
AXUIElement, and this framework is the sole public-facing avenue for programmatic manipulation, offering functions such as AXUIElementSetAttributeValue for changing a window's position or size.8 To use these functions, an application must first have explicit Accessibility permissions, which can be checked using  
AXIsProcessTrusted.7

### **1.2. The Core Challenge: Lack of a Public CGWindowID to AXUIElement Mapping**

The central problem for window management applications is the deliberate absence of a public API that directly bridges these two worlds. A developer can easily get a CGWindowID from Core Graphics and an AXUIElement from the Accessibility tree, but there is no sanctioned, public method to convert the CGWindowID into its corresponding AXUIElement.9 This architectural separation is a critical security and stability measure, ensuring that a high-level UI object cannot be tampered with by an unauthorized low-level process, and vice-versa.  
The kAXWindowIDAttribute was intended to solve this problem, promising a direct mapping from an AXUIElement to its CGWindowID. However, this attribute is an optional part of the Accessibility protocol and is notoriously unreliable. For many major applications, including Google Chrome, a request for this attribute returns an error, specifically kAXErrorCannotComplete (-25205).10 This is not an incidental bug but a consequence of how these applications are architected. The Chromium documentation reveals that its internal accessibility tree is a custom, multi-process data structure that is then bridged to native APIs.10 The incomplete or incorrect implementation of the native macOS protocol by these applications means that the  
kAXWindowIDAttribute is simply not reliably exposed. This architectural incompatibility renders the attribute unusable for any robust, general-purpose window management application.  
This analysis confirms that the issue is not a flaw in the developer's approach but a fundamental limitation of the public APIs. Therefore, any viable solution must circumvent this problem by relying on a hybrid strategy that correlates information from both frameworks without a direct API bridge.

## **2\. In-Depth Analysis of Existing Window Identification Methods**

The failure of the kAXWindowIDAttribute necessitates a closer examination of alternative identification methods. A single, perfect solution does not exist in the public API space. Instead, a successful strategy must acknowledge the limitations of each method and create a layered approach that provides redundancy and resilience.

### **2.1. Core Graphics Window IDs: Reliable Enumeration, Limits for Manipulation**

The CGWindowListCopyWindowInfo function is the most reliable way to retrieve information about on-screen windows. It provides an array of dictionaries containing a wealth of physical and process-related data.1 The  
CGWindowID is a unique and persistent handle for a window, while the kCGWindowOwnerPID links the window to its owning process. This combination is the most stable and performant way to build a foundational list of all manageable windows. However, this method is purely for gathering information, not for acting on it.

### **2.2. The Accessibility API: A Realm of Promise and Persistent Errors**

Within the Accessibility API, the primary function for attribute retrieval is AXUIElementCopyAttributeValue.7 While  
kAXWindowIDAttribute is the most direct approach, its documented unreliability for applications like Chrome mandates an exploration of other attributes. The kAXTitleAttribute is a tempting alternative, but it is highly dynamic. For web browsers, the title changes with every webpage, and for document-based applications, it changes with the filename.11 Such a dynamic attribute is fundamentally unsuitable for creating a persistent window handle.  
More stable, structured attributes exist. kAXRoleAttribute and kAXSubroleAttribute are less prone to change and are excellent for filtering. A window element almost always has the role AXWindow and a subrole like AXStandardWindow or AXFloatingWindow.13 The  
kAXIdentifierAttribute is intended for unique identification within an application's hierarchy and can be a strong signal for well-behaved applications, though its presence is not guaranteed.

### **2.3. Heuristic Approaches: The Inherent Flaws**

Heuristic methods, such as position-based matching, are inherently brittle. Matching windows based on their kCGWindowBounds and kAXPositionAttribute values is a valid approach, but it fails completely when windows are stacked or when their positions change due to screen reconfigurations.15 A combination of heuristics, such as using  
AXUIElementCopyElementAtPosition to perform a "hit-test" based on coordinates, can provide a coarse match but is too unreliable to be a primary identification strategy.16  
This analysis makes it clear that no single attribute or method provides a complete solution. A successful identification strategy must be a composite one, combining the strengths of the CG and AX APIs while mitigating their individual weaknesses. The following table provides a clear comparison of the primary window attributes.

| Attribute | Purpose | Reliability | Persistence | Availability |
| :---- | :---- | :---- | :---- | :---- |
| CGWindowID | Physical identifier for CG APIs | High | Persistent for window's lifetime | High (via CGWindowList) |
| kCGWindowOwnerPID | Links window to owning process | High | Persistent for window's lifetime | High (via CGWindowList) |
| kAXWindowIDAttribute | Bridges AX to CG | Low | Persistent if available | Low (unreliable for many apps) |
| kAXTitleAttribute | Semantic title for display | Low | Dynamic, changes with content | High (via AX) |
| kAXRoleAttribute | Semantic role of element | High | Stable | High (via AX) |
| kAXIdentifierAttribute | Unique within app hierarchy | Varies by application | Persistent if available | Varies by application |
| Bounds | Physical position and size | Low | Dynamic, changes with movement | High (via CG and AX) |

## **3\. Strategies from the Field: A Comparative Study of Major Window Managers**

The challenges of macOS window identification are well-known within the developer community. An examination of how other successful window managers address these problems provides crucial context and validates the need for a non-trivial approach. The strategies employed by these applications can be categorized along a spectrum of risk and control, ranging from security compromises for maximum functionality to a public API-only approach with known limitations.

### **3.1. The "SIP-Disabled" Approach: An Examination of Yabai's Architecture**

yabai is a highly performant and feature-rich tiling window manager for macOS.17 Its deep level of control over the window server is achieved through a controversial method: it requires users to  
**partially disable System Integrity Protection (SIP)**.17 SIP is a security feature that protects core system files and processes from modification, even by the root user.19  
yabai needs to inject a "scripting addition" into Dock.app, a process that is protected by SIP. The reason for this is that Dock.app is the "sole owner of the main connection to the window server".17 By injecting code into this protected process,  
yabai gains unparalleled, low-level access to control windows, spaces, and displays. This approach bypasses the limitations of public APIs entirely, granting yabai a degree of control that is otherwise impossible. While this provides exceptional functionality, it comes at a significant cost: it requires the user to weaken a fundamental security feature of their operating system, which is an unacceptable trade-off for a general-purpose application.20

### **3.2. The "Secure-ish" Compromise: An Investigation into AeroSpace's Single Private API**

AeroSpace is another tiling window manager that explicitly markets itself as **not requiring SIP to be disabled**.20 This represents a significant step up in security compared to the  
yabai model. The documentation for AeroSpace reveals a critical detail about its design: it uses a single, undocumented private API, \_AXUIElementGetWindow, to get a CGWindowID from an AXUIElement.21 This provides the exact public-API-bridging functionality that developers seek, without the need for a SIP bypass.  
This approach is a middle ground. It is more secure than disabling SIP but still relies on an undocumented API. The risk is that a future macOS update could change or remove this private API without notice, breaking the application's core functionality.21 This is a viable but precarious solution that is vulnerable to changes in the operating system's internal architecture.

### **3.3. The "Standard" Hybrid: A Look at Rectangle's Public API-Based Strategy**

Rectangle is a popular, open-source window manager that demonstrates the viability of a **public API-only strategy**.22 Written in Swift, it relies on a well-engineered hybrid approach that uses accessibility APIs and event taps to manage windows.22 The trade-off is that it cannot perform certain privileged operations, such as moving windows between different desktops or spaces, as Apple has not provided a public API for this.22  
The success of Rectangle demonstrates that it is possible to build a robust and functional application without resorting to private APIs or security compromises. This approach is the most stable and future-proof. It may have limitations compared to yabai but provides a level of security and reliability that is essential for a publicly distributed application.  
The following table provides a high-level comparison of these different strategies.

| Strategy | yabai (SIP Bypass) | AeroSpace (Private API) | Rectangle (Public APIs) | Proposed Strategy |
| :---- | :---- | :---- | :---- | :---- |
| **SIP Requirement** | Partial disablement | No | No | No |
| **Primary API Usage** | Scripting additions to Dock.app | AX, CG, single private API | AX, CG | AX, CG, PID correlation |
| **Reliability** | High | High (for now) | High | High |
| **Future-Proofing** | Low (dependent on SIP/Dock changes) | Low (dependent on private API) | High | High |
| **Trade-offs** | Functionality over security | Functionality over long-term stability | Stability over complete control | Stability & security over a single, simple API call |

## **4\. The Recommended Solution: A Tiered "Window Fingerprint" Strategy**

The analysis of macOS architecture and the strategies of other window managers leads to a clear conclusion: a viable and secure solution must be a multi-layered, hybrid approach. The recommended **Tiered Window Fingerprint Strategy** overcomes the limitations of any single identification method by creating a composite identifier that is highly unique and persistent.

### **4.1. Foundational Concepts: Defining the Window Fingerprint**

A "window fingerprint" is a collection of attributes from both the CG and AX APIs that, when combined, create a unique signature for a window. The goal is not to find a single perfect ID but to construct a probabilistic match that is reliable enough for a majority of use cases. The cornerstone of this strategy is the kCGWindowOwnerPID, which provides the critical link between the two otherwise disconnected frameworks. A CGWindowID can be linked to its process ID via CGWindowListCopyWindowInfo 2, and an  
AXUIElement can be linked to its process ID via AXUIElementGetPid or AXUIElementCreateApplication.7 This shared  
PID becomes the common denominator for correlation.

### **4.2. Tiered Identification Model and Fallback Mechanisms**

The proposed strategy operates on a tiered system, with each tier providing a fallback mechanism if the preceding tier fails to provide a reliable match.

* Tier 1: High-Reliability Identification.  
  This tier is designed to establish the most definitive match.  
  1. The process begins by using CGWindowListCopyWindowInfo to enumerate all on-screen windows and their properties, including the CGWindowID and the kCGWindowOwnerPID.  
  2. Next, the system uses AXUIElementCreateSystemWide() to get a system-wide accessibility object.7  
  3. The kAXWindowsAttribute of the system-wide object is requested, which returns an array of top-level window AXUIElements.14  
  4. For each window AXUIElement, the AXUIElementGetPid function is used to retrieve the Process ID.  
  5. The system then correlates the CGWindow objects from step 1 with the AXUIElements from step 4 by matching their Process IDs. This initial PID correlation significantly narrows the search space.  
  6. To achieve a unique match among windows with the same PID, the physical bounds of the window are compared. The kCGWindowBounds from the CG API is matched against the kAXPositionAttribute and kAXSizeAttribute of the AX API. While position-based matching alone is brittle, when combined with a confirmed PID match, it becomes a highly reliable method for establishing a one-to-one correlation.  
* Tier 2: Robust Fallback with Structured Attributes.  
  If the Tier 1 match is ambiguous or fails, the system falls back to a second tier of attributes. The kCGWindowOwnerName is combined with the more stable, semantic attributes from the AX tree, such as kAXRoleAttribute and kAXSubroleAttribute.13 A window owned by "Google Chrome" and having a role of  
  AXWindow is a strong candidate for a match. The kAXTitleAttribute can be used as a final filter within this tier, but with the understanding that it is highly dynamic and should be treated as a heuristic, not a unique identifier.11  
* Tier 3: The Heuristic Last Resort.  
  This tier is reserved for edge cases where the previous methods fail. The AXUIElementCopyElementAtPosition function can be used to perform a "hit-test" on a known coordinate from the kCGWindowBounds.16 This can provide an  
  AXUIElement for manipulation, but it is the least reliable method as it fails completely if a window's position is not unique or if other windows overlap it.

### **4.3. Maintaining State and Session Persistence**

To ensure this tiered strategy is performant and responsive, it requires a persistent in-memory cache. This cache should map each CGWindowID to its corresponding AXUIElement and window fingerprint. The application must monitor for window events (creation, destruction, movement, resizing) and update this cache accordingly. This ensures that the expensive initial enumeration and correlation process does not need to be repeated for every single operation, allowing for near-instantaneous window manipulation.

## **5\. Technical Implementation & Workarounds**

This section provides a practical, step-by-step guide for implementing the Tiered Window Fingerprint Strategy using Swift on macOS.

### **5.1. A Practical Swift Implementation**

A primary challenge in this implementation is handling the CGWindowList data structure, which is a CFArrayRef containing CFDictionaryRef objects. In Swift, this can be managed with CFBridging to convert the data into native Swift types like \`\`.  
The initial setup would involve a function to retrieve all on-screen windows:

Swift

import CoreGraphics

func getAllWindows() \-\>\] {  
    let windowList \= CGWindowListCopyWindowInfo(.optionOnScreenOnly, kCGNullWindowID) as???  
    return windowList as?\]??  
}

This function provides the raw data needed to begin the correlation process.

### **5.2. Bridging the CG-AX Gap: The Search and Correlate Method**

The core of the strategy is the correlation loop, which can be implemented as follows:

1. **Enumerate CG Windows**: Iterate through the results of getAllWindows() to get each window's CGWindowID, kCGWindowOwnerPID, and kCGWindowBounds.  
2. **Enumerate AX Windows**: For each unique kCGWindowOwnerPID, create an AXUIElement for the application using AXUIElementCreateApplication(pid: pid). Then, recursively traverse the application's accessibility tree to find all elements with kAXRoleAttribute equal to "AXWindow."  
3. **Correlate by PID and Bounds**: The PID match from steps 1 and 2 is the initial filter. For all CGWindow objects and AXUIElements with a matching PID, perform a geometric comparison. The kCGWindowBounds from the CGWindow dictionary is a rectangle, while the AXUIElement provides its position and size as a CGPoint and CGSize, respectively.15 A direct comparison of these two geometric representations will confirm a match.

This process builds the essential in-memory map, allowing subsequent window manipulation requests to immediately look up the corresponding AXUIElement from a known CGWindowID.

### **5.3. Handling Edge Cases: Minimized, Full-Screen, and Tabbed Windows**

The strategy must account for various window states. Minimized windows, for instance, are a special case. Their kCGWindowBounds will be different, and they may have a kAXMinimizedAttribute set to true.14 The application's state-tracking cache must be updated to reflect this.  
Native macOS tabbed windows, such as those in Terminal or Finder, present a unique challenge. They are based on the NSDocument model and may not behave as expected with a window manager.17 The recommended approach is to either make these windows  
float using rules or to recommend alternative applications that do not rely on this native tab system, as yabai does.17

## **6\. Application-Specific Deep Dives: Overcoming Known Obstacles**

The proposed strategy is a general-purpose solution, but certain applications, particularly those not built with native Apple frameworks, exhibit unique behaviors that require tailored workarounds.

### **6.1. Google Chrome: Analyzing its Accessibility Tree and Exploiting Workarounds**

The core issue with Chrome is the lack of a reliable AXWindowID.10 Chrome's accessibility implementation is rooted in its Chromium web-based architecture, which translates its internal accessibility tree to the native API.10 The key to managing Chrome windows is to rely exclusively on the Tier 1 strategy: correlate by  
PID and bounds.25 The  
kAXTitleAttribute is highly dynamic, but once a window is identified and its AXUIElement is cached, the AXTitle can be retrieved and displayed to the user for reference (e.g., displaying the webpage title) without being used as the primary identifier. The AXWebArea role can also be used to confirm that an element is indeed a web content area, which can be helpful for filtering.10

### **6.2. Terminal and VS Code: Patterns in Native and Cross-Platform Behavior**

Terminal.app, as a native Apple application, is a "best-case" scenario. It is expected to have a complete and well-formed accessibility tree, making it a perfect test case for the Tier 1 strategy.26 The  
PID and bounds correlation will function reliably, and all manipulation attributes will be available.  
VS Code, on the other hand, is built on the Electron framework, which is a web view-based technology. Its behavior is analogous to Chrome's. It is likely to fail on AXWindowID and will require the same hybrid PID \+ bounds correlation strategy.27 The report recommends the Tier 1 strategy for this application as well, proving that the proposed general-purpose approach is effective for both native and cross-platform applications.  
The following table demonstrates how the tiered strategy is applied to different applications.

| Application | Primary Strategy (Tier 1\) | Fallback Strategy (Tier 2/3) | Specific Notes |
| :---- | :---- | :---- | :---- |
| Terminal.app | PID \+ Bounds Correlation | Not needed | kAXRole and kAXSubrole are reliable. |
| Google Chrome | PID \+ Bounds Correlation | AXTitle as a heuristic marker | AXWindowID fails. Title is dynamic. |
| VS Code | PID \+ Bounds Correlation | AXTitle as a heuristic marker | AXWindowID likely fails. Based on web view architecture. |
| Finder | PID \+ Bounds Correlation | AXTitle as a heuristic marker | NSDocument-based tabs may be problematic. |

## **7\. Future-Proofing for macOS Sequoia and Beyond**

As Apple continues to evolve macOS, it is essential to consider the long-term viability of any proposed solution. The analysis of recent and upcoming macOS releases provides crucial context for this discussion.

### **7.1. Analysis of macOS 15.6 API Changes (Accessibility & Core Graphics)**

A review of the macOS Sequoia release notes reveals that Apple is investing in its native window management capabilities, such as "easier window tiling" and new keyboard shortcuts.29 However, there is no indication that Apple plans to expose new public APIs to allow third-party developers to replicate or extend these features. This reinforces the central premise of this report: the architectural separation between the CG and AX APIs for window management is a deliberate design choice, not a temporary limitation that will be fixed with a new API.  
The ScreenCaptureKit framework, introduced in macOS 12.3, provides a public way to bridge a CGWindowID to an SCWindow object.30 However, the purpose of this framework is explicitly for screen capture, and it does not provide any functions for window manipulation.4 The existence of this framework, and its limited scope, further confirms that Apple is intentionally not providing a general-purpose  
CGWindowID to AXUIElement mapping for third-party window managers.

### **7.2. The Long-Term Outlook for Third-Party Window Managers on macOS**

The situation for third-party window managers remains a "cat-and-mouse" game with the operating system. The AeroSpace model, which relies on a single private API, is inherently fragile and at risk of breaking with any future macOS update.21 The  
yabai approach, while offering maximum control, is fundamentally insecure due to its reliance on disabling SIP.17  
The recommended **Tiered Window Fingerprint Strategy**, while more complex to implement, is the only truly robust and future-proof path. It relies exclusively on stable, public APIs and a well-defined correlation logic. By not depending on private APIs or security compromises, this approach is the most resilient to future macOS changes and provides the most dependable foundation for a long-term development effort.

## **8\. Conclusion & Final Recommendations**

This report confirms that the problem of reliable window identification on macOS is rooted in a deliberate architectural separation between Core Graphics and the Accessibility API. The kAXWindowIDAttribute is not a reliable solution, and heuristic approaches are inherently brittle.  
The most effective strategy is a multi-layered, hybrid model that correlates information from both the CG and AX frameworks using a tiered approach. The cornerstone of this strategy is the PID, which serves as a common denominator to link a CGWindowID with an AXUIElement. This approach, combined with a persistent in-memory cache, offers a robust and performant solution that is resilient to the architectural limitations of the operating system and the behavioral quirks of cross-platform applications.  
The final recommendation is to proceed with the development of the window stacking application using the **Tiered Window Fingerprint Strategy**. The implementation should focus on the Tier 1 PID \+ bounds correlation as the primary method, with the Tier 2 and Tier 3 heuristics serving as a solid fallback. This strategy is the most secure and reliable path forward, offering the best balance of functionality, performance, and long-term stability without compromising user security or violating Apple's developer guidelines.

#### **Works cited**

1. CGWindowListCreate \- Documentation \- Apple Developer, accessed August 28, 2025, [https://developer.apple.com/documentation/coregraphics/1552209-cgwindowlistcreate?language=objc](https://developer.apple.com/documentation/coregraphics/1552209-cgwindowlistcreate?language=objc)  
2. How to identify which process is running which window in Mac OS X? \- Super User, accessed August 28, 2025, [https://superuser.com/questions/902869/how-to-identify-which-process-is-running-which-window-in-mac-os-x](https://superuser.com/questions/902869/how-to-identify-which-process-is-running-which-window-in-mac-os-x)  
3. CGWindowID | Apple Developer Documentation, accessed August 28, 2025, [https://developer.apple.com/documentation/coregraphics/cgwindowid](https://developer.apple.com/documentation/coregraphics/cgwindowid)  
4. Capturing screen content in macOS | Apple Developer Documentation, accessed August 28, 2025, [https://developer.apple.com/documentation/screencapturekit/capturing-screen-content-in-macos?changes=\_9](https://developer.apple.com/documentation/screencapturekit/capturing-screen-content-in-macos?changes=_9)  
5. Accessibility API | Apple Developer Documentation, accessed August 28, 2025, [https://developer.apple.com/documentation/accessibility/accessibility-api](https://developer.apple.com/documentation/accessibility/accessibility-api)  
6. Accessibility and UI Testing \- GitHub Gist, accessed August 28, 2025, [https://gist.github.com/funmia/9ecc9bf4c135e49a3902db8d23c2a846](https://gist.github.com/funmia/9ecc9bf4c135e49a3902db8d23c2a846)  
7. AXUIElement.h \- Documentation \- Apple Developer, accessed August 28, 2025, [https://developer.apple.com/documentation/applicationservices/axuielement\_h](https://developer.apple.com/documentation/applicationservices/axuielement_h)  
8. AXUIElementCopyAttributeValues(\_:\_:\_:\_:\_:) | Apple Developer Documentation, accessed August 28, 2025, [https://developer.apple.com/documentation/applicationservices/1462060-axuielementcopyattributevalues](https://developer.apple.com/documentation/applicationservices/1462060-axuielementcopyattributevalues)  
9. How to create an AXUIElementRef from an NSView or NSWindow? \- Stack Overflow, accessed August 28, 2025, [https://stackoverflow.com/questions/39906540/how-to-create-an-axuielementref-from-an-nsview-or-nswindow](https://stackoverflow.com/questions/39906540/how-to-create-an-axuielementref-from-an-nsview-or-nswindow)  
10. Chromium Docs \- Accessibility Overview, accessed August 28, 2025, [https://chromium.googlesource.com/chromium/src/+/main/docs/accessibility/overview.md](https://chromium.googlesource.com/chromium/src/+/main/docs/accessibility/overview.md)  
11. Chrome: Any way to assign sticky window name \- Super User, accessed August 28, 2025, [https://superuser.com/questions/908196/chrome-any-way-to-assign-sticky-window-name](https://superuser.com/questions/908196/chrome-any-way-to-assign-sticky-window-name)  
12. Understanding Success Criterion 2.4.2: Page Titled | WAI \- W3C, accessed August 28, 2025, [https://www.w3.org/WAI/WCAG22/Understanding/page-titled.html](https://www.w3.org/WAI/WCAG22/Understanding/page-titled.html)  
13. Mac "AX" property and value names in AAM specs · Issue \#399 · w3c/aria \- GitHub, accessed August 28, 2025, [https://github.com/w3c/aria/issues/399](https://github.com/w3c/aria/issues/399)  
14. Attributes | Apple Developer Documentation, accessed August 28, 2025, [https://developer.apple.com/documentation/applicationservices/carbon\_accessibility/attributes](https://developer.apple.com/documentation/applicationservices/carbon_accessibility/attributes)  
15. Move other windows on Mac OS X using Accessibility API \- Stack Overflow, accessed August 28, 2025, [https://stackoverflow.com/questions/21069066/move-other-windows-on-mac-os-x-using-accessibility-api](https://stackoverflow.com/questions/21069066/move-other-windows-on-mac-os-x-using-accessibility-api)  
16. AXUIElementCopyElementAtPos, accessed August 28, 2025, [https://developer.apple.com/documentation/applicationservices/1462077-axuielementcopyelementatposition](https://developer.apple.com/documentation/applicationservices/1462077-axuielementcopyelementatposition)  
17. koekeishiya/yabai: A tiling window manager for macOS based on binary space partitioning, accessed August 28, 2025, [https://github.com/koekeishiya/yabai](https://github.com/koekeishiya/yabai)  
18. yabai download | SourceForge.net, accessed August 28, 2025, [https://sourceforge.net/projects/yabai.mirror/](https://sourceforge.net/projects/yabai.mirror/)  
19. Disabling System Integrity Protection · koekeishiya/yabai Wiki · GitHub, accessed August 28, 2025, [https://github.com/koekeishiya/yabai/wiki/Disabling-System-Integrity-Protection/0b75fef379c579096b2a51d6a0ddeb92ebe12130](https://github.com/koekeishiya/yabai/wiki/Disabling-System-Integrity-Protection/0b75fef379c579096b2a51d6a0ddeb92ebe12130)  
20. Doesn't require disabling SIP That's very interesting\! I've been hesitant to u... \- Hacker News, accessed August 28, 2025, [https://news.ycombinator.com/item?id=40598013](https://news.ycombinator.com/item?id=40598013)  
21. AeroSpace is an i3-like tiling window manager for macOS \- GitHub, accessed August 28, 2025, [https://github.com/nikitabobko/AeroSpace](https://github.com/nikitabobko/AeroSpace)  
22. rxhanson/Rectangle: Move and resize windows on macOS with keyboard shortcuts and snap areas \- GitHub, accessed August 28, 2025, [https://github.com/rxhanson/Rectangle](https://github.com/rxhanson/Rectangle)  
23. AXUIElementCopyAttributeNames \- Documentation \- Apple Developer, accessed August 28, 2025, [https://developer.apple.com/documentation/applicationservices/1459475-axuielementcopyattributenames?language=objc](https://developer.apple.com/documentation/applicationservices/1459475-axuielementcopyattributenames?language=objc)  
24. AXUIElementCopyElementAtPos, accessed August 28, 2025, [https://developer.apple.com/documentation/applicationservices/1462077-axuielementcopyelementatposition?language=objc](https://developer.apple.com/documentation/applicationservices/1462077-axuielementcopyelementatposition?language=objc)  
25. How to retrieve active window URL using Mac OS X accessibility API \- Stack Overflow, accessed August 28, 2025, [https://stackoverflow.com/questions/53229924/how-to-retrieve-active-window-url-using-mac-os-x-accessibility-api](https://stackoverflow.com/questions/53229924/how-to-retrieve-active-window-url-using-mac-os-x-accessibility-api)  
26. Use macOS accessibility features in Terminal on Mac \- Apple Support, accessed August 28, 2025, [https://support.apple.com/guide/terminal/use-macos-accessibility-features-trml1020/mac](https://support.apple.com/guide/terminal/use-macos-accessibility-features-trml1020/mac)  
27. VS Code API | Visual Studio Code Extension API, accessed August 28, 2025, [https://code.visualstudio.com/api/references/vscode-api](https://code.visualstudio.com/api/references/vscode-api)  
28. Visual Studio Code for the Web, accessed August 28, 2025, [https://code.visualstudio.com/docs/setup/vscode-web](https://code.visualstudio.com/docs/setup/vscode-web)  
29. What's new in the updates for macOS Sequoia \- Apple Support, accessed August 28, 2025, [https://support.apple.com/en-us/120283](https://support.apple.com/en-us/120283)  
30. windowID | Apple Developer Documentation, accessed August 28, 2025, [https://developer.apple.com/documentation/screencapturekit/scwindow/windowid](https://developer.apple.com/documentation/screencapturekit/scwindow/windowid)  
31. SCWindow | Apple Developer Documentation, accessed August 28, 2025, [https://developer.apple.com/documentation/screencapturekit/scwindow](https://developer.apple.com/documentation/screencapturekit/scwindow)  
32. ScreenCaptureKit | Apple Developer Documentation, accessed August 28, 2025, [https://developer.apple.com/documentation/screencapturekit/](https://developer.apple.com/documentation/screencapturekit/)