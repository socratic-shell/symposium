import SwiftUI

struct ActiveProjectView: View {
    let project: Project
    let onCloseProject: () -> Void
    
    var body: some View {
        VStack(spacing: 24) {
            // Project info
            VStack(spacing: 16) {
                Image(systemName: "folder")
                    .font(.system(size: 48))
                    .foregroundColor(.accentColor)
                
                VStack(spacing: 8) {
                    Text(project.name)
                        .font(.title)
                        .fontWeight(.semibold)
                    
                    Text(project.directoryPath)
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .textSelection(.enabled)
                }
            }
            
            // Status
            VStack(spacing: 12) {
                HStack {
                    Image(systemName: "circle.fill")
                        .foregroundColor(.green)
                        .font(.caption)
                    Text("Project Active")
                        .font(.headline)
                        .foregroundColor(.primary)
                }
                
                Text("Click the dock icon to access taskspaces and project tools.")
                    .font(.body)
                    .foregroundColor(.secondary)
                    .multilineTextAlignment(.center)
            }
            
            Spacer()
            
            // Actions
            VStack(spacing: 12) {
                Button("Close Project") {
                    onCloseProject()
                }
                .buttonStyle(.borderedProminent)
                .controlSize(.large)
                
                Text("Closing the project will return you to project selection.")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .multilineTextAlignment(.center)
            }
        }
        .padding(32)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

