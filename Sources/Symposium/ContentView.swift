import SwiftUI
import AppKit

struct ContentView: View {
    @ObservedObject var windowManager: WindowManager
    
    var body: some View {
        VStack(spacing: 20) {
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
            
            // Available windows section
            VStack {
                HStack {
                    Text("Available Windows")
                        .font(.headline)
                    Spacer()
                    Button("Refresh") {
                        windowManager.refreshWindowList()
                    }
                }
                
                ScrollView {
                    ForEach(windowManager.allWindows) { window in
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
        }
        .padding()
    }
}