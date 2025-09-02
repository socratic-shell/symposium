import SwiftUI
import AppKit

struct ProjectSelectionView: View {
    @EnvironmentObject var permissionManager: PermissionManager
    @EnvironmentObject var agentManager: AgentManager
    @EnvironmentObject var settingsManager: SettingsManager
    let onProjectCreated: (ProjectManager) -> Void
    @State private var showingNewProjectDialog = false
    @State private var showingOpenProjectDialog = false
    
    private var hasValidAgent: Bool {
        agentManager.availableAgents.first(where: { $0.id == settingsManager.selectedAgent })?.isInstalled == true &&
        agentManager.availableAgents.first(where: { $0.id == settingsManager.selectedAgent })?.isMCPConfigured == true
    }
    
    private var hasRequiredPermissions: Bool {
        permissionManager.hasAccessibilityPermission && permissionManager.hasScreenRecordingPermission
    }
    
    private var canCreateProjects: Bool {
        hasValidAgent && hasRequiredPermissions
    }
    
    var body: some View {
        VStack(spacing: 24) {
            // Header
            VStack(spacing: 8) {
                Image(systemName: "folder.badge.gearshape")
                    .font(.system(size: 48))
                    .foregroundColor(.blue)
                
                Text("Symposium")
                    .font(.largeTitle)
                    .fontWeight(.bold)
                
                Text("Select or create a project to get started")
                    .font(.headline)
                    .foregroundColor(.secondary)
            }
            
            // Action buttons
            VStack(spacing: 16) {
                Button(action: { showingNewProjectDialog = true }) {
                    HStack {
                        Image(systemName: "plus.circle.fill")
                        Text("Create New Project")
                    }
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(canCreateProjects ? Color.blue : Color.gray)
                    .foregroundColor(.white)
                    .cornerRadius(8)
                }
                .disabled(!canCreateProjects)
                
                Button(action: { 
                    Logger.shared.log("Open Existing Project button clicked")
                    showingOpenProjectDialog = true 
                    Logger.shared.log("Set showingOpenProjectDialog to true")
                }) {
                    HStack {
                        Image(systemName: "folder.circle")
                        Text("Open Existing Project")
                    }
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(canCreateProjects ? Color.gray.opacity(0.2) : Color.gray.opacity(0.1))
                    .foregroundColor(canCreateProjects ? .primary : .secondary)
                    .cornerRadius(8)
                }
                .disabled(!canCreateProjects)
            }
            .frame(maxWidth: 300)
            
            // Status message when not ready
            if !canCreateProjects {
                VStack(spacing: 8) {
                    HStack {
                        Image(systemName: "exclamationmark.triangle.fill")
                            .foregroundColor(.orange)
                        Text("Setup Required")
                            .font(.headline)
                            .foregroundColor(.orange)
                    }
                    
                    VStack(alignment: .leading, spacing: 4) {
                        if !hasRequiredPermissions {
                            Text("• Missing required permissions")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                        if !hasValidAgent {
                            Text("• No valid AI agent configured")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                        Text("Open Settings to complete setup")
                            .font(.caption)
                            .foregroundColor(.blue)
                    }
                }
                .padding()
                .background(Color.orange.opacity(0.1))
                .cornerRadius(8)
            }
            
            Spacer()
        }
        .padding(40)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .sheet(isPresented: $showingNewProjectDialog) {
            NewProjectDialog(onProjectCreated: onProjectCreated)
        }
        .sheet(isPresented: $showingOpenProjectDialog) {
            OpenProjectDialog(onProjectCreated: onProjectCreated)
        }
        .onAppear {
            Logger.shared.log("ProjectSelectionView appeared")
            agentManager.scanForAgents()
        }
    }
}

struct NewProjectDialog: View {
    @EnvironmentObject var agentManager: AgentManager
    @EnvironmentObject var settingsManager: SettingsManager
    let onProjectCreated: (ProjectManager) -> Void
    @Environment(\.dismiss) private var dismiss
    
    @State private var projectName = ""
    @State private var gitURL = ""
    @State private var selectedDirectory = ""
    @State private var showingDirectoryPicker = false
    
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Create New Project")
                .font(.headline)
            
            VStack(alignment: .leading, spacing: 8) {
                Text("Project Name:")
                TextField("Enter project name", text: $projectName)
                    .textFieldStyle(RoundedBorderTextFieldStyle())
            }
            
            VStack(alignment: .leading, spacing: 8) {
                Text("Git Repository URL:")
                TextField("https://github.com/user/repo.git", text: $gitURL)
                    .textFieldStyle(RoundedBorderTextFieldStyle())
            }
            
            VStack(alignment: .leading, spacing: 8) {
                Text("Location:")
                HStack {
                    Text(selectedDirectory.isEmpty ? "Select directory..." : selectedDirectory)
                        .foregroundColor(selectedDirectory.isEmpty ? .secondary : .primary)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(8)
                        .background(Color.gray.opacity(0.1))
                        .cornerRadius(4)
                    
                    Button("Browse") {
                        showingDirectoryPicker = true
                    }
                }
            }
            
            HStack {
                Button("Cancel") {
                    dismiss()
                }
                
                Spacer()
                
                Button("Create") {
                    createProject()
                }
                .disabled(projectName.isEmpty || gitURL.isEmpty || selectedDirectory.isEmpty)
            }
        }
        .padding()
        .frame(width: 400)
        .fileImporter(
            isPresented: $showingDirectoryPicker,
            allowedContentTypes: [.folder],
            allowsMultipleSelection: false
        ) { result in
            switch result {
            case .success(let urls):
                if let url = urls.first {
                    selectedDirectory = url.path
                }
            case .failure(let error):
                // Could show alert here
                print("Failed to select directory: \(error.localizedDescription)")
            }
        }
    }
    
    private func createProject() {
        let projectManager = ProjectManager(agentManager: agentManager, selectedAgent: settingsManager.selectedAgent)
        do {
            try projectManager.createProject(name: projectName, gitURL: gitURL, at: selectedDirectory)
            onProjectCreated(projectManager)
            dismiss()
        } catch {
            // Handle error - could show alert
        }
    }
}

struct OpenProjectDialog: View {
    @EnvironmentObject var agentManager: AgentManager
    @EnvironmentObject var settingsManager: SettingsManager
    let onProjectCreated: (ProjectManager) -> Void
    @Environment(\.dismiss) private var dismiss
    
    @State private var showingDirectoryPicker = false
    
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Open Existing Project")
                .font(.headline)
            
            Text("Select a .symposium project directory:")
                .foregroundColor(.secondary)
            
            Button("Browse for Project Directory") {
                Logger.shared.log("Browse for Project Directory button clicked")
                showingDirectoryPicker = true
                Logger.shared.log("Set showingDirectoryPicker to true")
            }
            .frame(maxWidth: .infinity)
            .padding()
            .background(Color.blue)
            .foregroundColor(.white)
            .cornerRadius(8)
            
            HStack {
                Button("Cancel") {
                    dismiss()
                }
                
                Spacer()
            }
        }
        .padding()
        .frame(width: 400)
        .onAppear {
            Logger.shared.log("OpenProjectDialog appeared")
        }
        .fileImporter(
            isPresented: $showingDirectoryPicker,
            allowedContentTypes: [.folder],
            allowsMultipleSelection: false
        ) { result in
            switch result {
            case .success(let urls):
                Logger.shared.log("File picker succeeded with URLs: \(urls)")
                if let url = urls.first {
                    Logger.shared.log("Selected URL: \(url.path)")
                    openProject(at: url.path)
                } else {
                    Logger.shared.log("ERROR: No URL selected from file picker")
                }
            case .failure(let error):
                Logger.shared.log("ERROR: File picker failed: \(error)")
                // Could show alert here
                print("Failed to select directory: \(error.localizedDescription)")
            }
        }
    }
    
    private func openProject(at path: String) {
        Logger.shared.log("OpenProjectDialog.openProject called with path: \(path)")
        let projectManager = ProjectManager(agentManager: agentManager, selectedAgent: settingsManager.selectedAgent)
        Logger.shared.log("Created ProjectManager with selectedAgent: \(settingsManager.selectedAgent)")
        do {
            Logger.shared.log("Attempting to open project at: \(path)")
            try projectManager.openProject(at: path)
            Logger.shared.log("Successfully opened project, calling onProjectCreated callback")
            onProjectCreated(projectManager)
            Logger.shared.log("Called onProjectCreated, dismissing dialog")
            dismiss()
        } catch {
            Logger.shared.log("ERROR: Failed to open project: \(error)")
            // Handle error - could show alert
        }
    }
}
