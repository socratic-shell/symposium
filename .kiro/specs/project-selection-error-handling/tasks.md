# Implementation Plan

- [x] 1. Create error handling foundation

  - Create `ProjectValidationError` enum with localized error messages and recovery suggestions
  - Add the enum to the existing `Project.swift` file with proper Swift error handling protocols
  - _Requirements: 1.1, 2.2, 4.1_

- [x] 2. Enhance project validation logic

  - Extend `Project.swift` with `validateProjectDirectory()` method that returns detailed validation results
  - Replace boolean `isValidProjectDirectory()` usage with detailed validation where user feedback is needed
  - Maintain backward compatibility by keeping the existing boolean method for internal use
  - _Requirements: 1.1, 3.2, 4.2_

- [x] 3. Implement alert presentation utility

  - Create `ProjectValidationAlert` utility struct for consistent error presentation across the app
  - Use native `NSAlert` with warning style and appropriate button options
  - Include "Create New Project" recovery action that integrates with existing new project flow
  - _Requirements: 1.2, 2.1, 2.3, 2.4_

- [x] 4. Update ProjectSelectionView with immediate validation

  - Modify the `fileImporter` success handler in `ProjectSelectionView.swift` to validate before calling `openProject`
  - Integrate error alert presentation when validation fails
  - _Requirements: 1.1, 1.3, 3.1, 3.3_

- [ ] 5. Enhance App.swift fallback error handling

  - Update `runStartupLogic()` method to use detailed validation for previously opened projects
  - Ensure invalid project paths are cleared when validation fails during app startup
  - Maintain existing app state machine behavior while adding proper error logging
  - _Requirements: 1.4, 3.1, 3.2, 4.4_

- [ ] 6. Add notification support for recovery actions

  - Implement notification-based communication between alert and new project dialog
  - Ensure "Create New Project" button in error alerts properly triggers the new project flow
  - Test integration between error handling and existing project creation workflow
  - _Requirements: 1.2, 3.1_
