import SwiftUI
import AppKit

struct ContentView: View {
    @ObservedObject var windowManager: WindowManager
    @State private var searchText: String = ""
    
    var filteredWindows: [WindowManager.WindowInfo] {
        if searchText.isEmpty {
            return windowManager.allWindows
        } else {
            return windowManager.allWindows.filter { window in
                window.displayName.localizedCaseInsensitiveContains(searchText) ||
                window.appName.localizedCaseInsensitiveContains(searchText) ||
                window.title.localizedCaseInsensitiveContains(searchText)
            }
        }
    }
    
    var body: some View {
        VStack(spacing: 20) {
            // Status section
            VStack(spacing: 8) {
                HStack {
                    Image(systemName: windowManager.hasAccessibilityPermission ? "checkmark.circle.fill" : "exclamationmark.triangle.fill")
                        .foregroundColor(windowManager.hasAccessibilityPermission ? .green : .orange)
                    Text(windowManager.hasAccessibilityPermission ? "Accessibility: Enabled" : "Accessibility: Required")
                        .font(.caption)
                    Spacer()
                    if !windowManager.hasAccessibilityPermission {
                        Button("Grant Permission") {
                            windowManager.requestAccessibilityPermission()
                        }
                        .font(.caption)
                    }
                }
                
                if !windowManager.lastOperationMessage.isEmpty {
                    Text(windowManager.lastOperationMessage)
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .multilineTextAlignment(.leading)
                }
            }
            .padding(.horizontal)
            
            Divider()
            
            // Stack section
            VStack {
                Text("Stack (\(windowManager.stackedWindows.count) windows)")
                    .font(.headline)
                
                ScrollView {
                    ForEach(windowManager.stackedWindows) { window in
                        HStack {
                            Text(window.displayName)
                                .foregroundColor(
                                    windowManager.currentStackIndex == 
                                    windowManager.stackedWindows.firstIndex(where: { $0.id == window.id }) 
                                    ? .blue : .primary
                                )
                            Spacer()
                            Button("Remove") {
                                windowManager.removeFromStack(window)
                            }
                        }
                        .padding(.horizontal)
                    }
                }
                .frame(height: 150)
                .border(Color.gray)
                
                HStack {
                    Button("Previous") {
                        windowManager.previousWindow()
                    }
                    .disabled(windowManager.stackedWindows.isEmpty)
                    
                    Button("Next") {
                        windowManager.nextWindow()
                    }
                    .disabled(windowManager.stackedWindows.isEmpty)
                }
            }
            
            Divider()
            
            // Configuration section
            VStack {
                HStack {
                    Text("Stacking Configuration")
                        .font(.headline)
                    Spacer()
                }
                
                VStack(alignment: .leading, spacing: 8) {
                    HStack {
                        Text("Follower Inset:")
                            .font(.caption)
                        Spacer()
                        Text("\(Int(windowManager.insetPercentage * 100))%")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    
                    Slider(
                        value: Binding(
                            get: { windowManager.insetPercentage },
                            set: { windowManager.insetPercentage = $0 }
                        ),
                        in: 0.05...0.20,
                        step: 0.01
                    ) {
                        Text("Inset Percentage")
                    }
                    
                    Text("Controls how much smaller follower windows are relative to the leader.")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
                .padding(.horizontal)
            }
            
            Divider()
            
            // Available windows section
            VStack {
                HStack {
                    Text("Available Windows")
                        .font(.headline)
                    Spacer()
                    Button("Refresh") {
                        windowManager.checkAccessibilityPermission()
                        windowManager.refreshWindowList()
                    }
                }
                
                // Search field
                HStack {
                    Image(systemName: "magnifyingglass")
                        .foregroundColor(.secondary)
                    TextField("Filter windows...", text: $searchText)
                        .textFieldStyle(RoundedBorderTextFieldStyle())
                }
                .padding(.horizontal)
                
                ScrollView {
                    ForEach(filteredWindows) { window in
                        HStack {
                            Text(window.displayName)
                            Spacer()
                            Button("Add to Stack") {
                                windowManager.addToStack(window)
                            }
                        }
                        .padding(.horizontal)
                    }
                }
            }
            
            Divider()
            
            // Debug log section
            VStack {
                HStack {
                    Text("Debug Log")
                        .font(.headline)
                    Spacer()
                    Button("Copy") {
                        NSPasteboard.general.clearContents()
                        NSPasteboard.general.setString(windowManager.debugLog, forType: .string)
                    }
                    .font(.caption)
                    .disabled(windowManager.debugLog.isEmpty)
                    
                    Button("Clear") {
                        windowManager.clearLog()
                    }
                    .font(.caption)
                }
                
                ScrollView {
                    ScrollViewReader { proxy in
                        Text(windowManager.debugLog.isEmpty ? "No debug output yet..." : windowManager.debugLog)
                            .font(.system(.caption, design: .monospaced))
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding(8)
                            .background(Color.black.opacity(0.05))
                            .onChange(of: windowManager.debugLog) { _ in
                                // Auto-scroll to bottom when new log entries are added
                                withAnimation {
                                    proxy.scrollTo("bottom", anchor: .bottom)
                                }
                            }
                            .id("logContent")
                        
                        // Invisible element to enable auto-scroll to bottom
                        HStack { }
                            .id("bottom")
                    }
                }
                .frame(height: 150)
                .border(Color.gray)
            }
        }
        .padding()
    }
}