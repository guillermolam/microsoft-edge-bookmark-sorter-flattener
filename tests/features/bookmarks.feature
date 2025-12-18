Feature: Bookmarks CLI normalization

  Scenario: Normalize output validates
    Given a temp bookmarks workspace
    And an input bookmarks file with duplicates
    When I run bookmarks normalize to an output file
    Then the command succeeds
    When I run bookmarks validate on the output file
    Then the command succeeds

  Scenario: Normalize is deterministic
    Given a temp bookmarks workspace
    And an input bookmarks file with duplicates
    When I run bookmarks normalize twice to two output files
    Then the two outputs are identical

  Scenario: Overwriting without backup is refused
    Given a temp bookmarks workspace
    And an input bookmarks file with duplicates
    When I run bookmarks normalize in place without backup
    Then the command fails
    And stderr mentions "--backup"

  Scenario: Overwriting with backup creates a timestamped backup
    Given a temp bookmarks workspace
    And an input bookmarks file with duplicates
    When I run bookmarks normalize in place with backup
    Then the command succeeds
    And a timestamped backup file exists

  @requires_real_bookmarks
  Scenario: Real Bookmarks final state validates (no duplicates, terminates)
    Given a temp bookmarks workspace
    And a bookmarks file from env "EDGE_BOOKMARKS_PATH"
    When I run bookmarks normalize to an output file
    Then the command succeeds
    And output has no duplicate folder names
    And output has no duplicate urls within any folder
    When I run bookmarks validate on the output file
    Then the command succeeds

  @requires_real_bookmarks
  Scenario: Real Bookmarks schema validation and dry run
    Given a temp bookmarks workspace
    And a bookmarks file from env "EDGE_BOOKMARKS_PATH"
    When I run bookmarks validate on the input file
    Then the command succeeds
    And stderr mentions "schema validation passed"
    When I run bookmarks normalize with dry run on the input file
    Then the command succeeds
    And no output file is created
