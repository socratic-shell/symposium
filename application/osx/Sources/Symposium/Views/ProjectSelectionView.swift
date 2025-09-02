import SwiftUI
import AppKit

struct ProjectSelectionView: View {
    @StateObject private var projectManager = ProjectManager()
    @State private var showingNewProjectDialog = false
    @State private var showingOpenProjectDialog = false
    
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
                    .background(Color.blue)
                    .foregroundColor(.white)
                    .cornerRadius(8)
                }
                
                Button(action: { showingOpenProjectDialog = true }) {
                    HStack {
                        Image(systemName: "folder.circle")
                        Text("Open Existing Project")
                    }
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(Color.gray.opacity(0.2))
                    .foregroundColor(.primary)
                    .cornerRadius(8)
                }
            }
            .frame(maxWidth: 300)
            
            // Error message
            if let errorMessage = projectManager.errorMessage {
                Text(errorMessage)
                    .foregroundColor(.red)
                    .padding()
                    .background(Color.red.opacity(0.1))
                    .cornerRadius(8)
            }
            
            Spacer()
        }
        .padding(40)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .sheet(isPresented: $showingNewProjectDialog) {
            NewProjectDialog(projectManager: projectManager)
        }
        .sheet(isPresented: $showingOpenProjectDialog) {
            OpenProjectDialog(projectManager: projectManager)
        }
    }
}

struct NewProjectDialog: View {
    @ObservedObject var projectManager: ProjectManager
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
                projectManager.setError("Failed to select directory: \(error.localizedDescription)")
            }
        }
    }
    
    private func createProject() {
        do {
            try projectManager.createProject(name: projectName, gitURL: gitURL, at: selectedDirectory)
            dismiss()
        } catch {
            projectManager.setError(error.localizedDescription)
        }
    }
}

struct OpenProjectDialog: View {
    @ObservedObject var projectManager: ProjectManager
    @Environment(\.dismiss) private var dismiss
    
    @State private var showingDirectoryPicker = false
    
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Open Existing Project")
                .font(.headline)
            
            Text("Select a .symposium project directory:")
                .foregroundColor(.secondary)
            
            Button("Browse for Project Directory") {
                showingDirectoryPicker = true
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
        .fileImporter(
            isPresented: $showingDirectoryPicker,
            allowedContentTypes: [.folder],
            allowsMultipleSelection: false
        ) { result in
            switch result {
            case .success(let urls):
                if let url = urls.first {
                    openProject(at: url.path)
                }
            case .failure(let error):
                projectManager.setError("Failed to select directory: \(error.localizedDescription)")
            }
        }
    }
    
    private func openProject(at path: String) {
        do {
            try projectManager.openProject(at: path)
            dismiss()
        } catch {
            projectManager.setError(error.localizedDescription)
        }
    }
}
