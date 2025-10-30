import type {
  BabelCodeFrameOptions,
  SourceLocation,
} from 'next/dist/compiled/babel/code-frame'

// Don't render a codeframe if the file is larger than this.
// This is not a great heuristic.  Ideally codeframe would be smarter about handling long lines or large files
const MAX_CODEFRAME_FILESIZE = 5 * 1024 * 1024
// 1MB. There are handwritten files larger than this but regressing on syntax highlighting in those cases should be fine.
const MAX_HIGHLIGHT_FILESIZE = 1024 * 1024
// Don't render a codeframe if the column is larger than this.
const MAX_COLUMN = 300

/**
 * Renders a babel code frame defensively
 *
 * Takes care to:
 * - avoid syntax highlighting large files which can trigger errors due to limitations in the regex tokenizer inside of babel
 * - avoid rendering a
 * @param file
 * @param location
 * @param options
 */
export function codeFrameColumns(
  file: string,
  location: SourceLocation,
  options?: BabelCodeFrameOptions
): string {
  if (
    file.length > MAX_CODEFRAME_FILESIZE ||
    (location.start.column ?? 0) > MAX_COLUMN ||
    (location.end?.column ?? 0) > MAX_COLUMN
  ) {
    // either the file or the column is too big for a code frame .
    return ''
  }
  options ??= {}
  if (file.length > MAX_HIGHLIGHT_FILESIZE) {
    delete options.forceColor
    options.highlightCode = false
  }
  const { codeFrameColumns: babelCodeFrameColumns } =
    require('next/dist/compiled/babel/code-frame') as typeof import('next/dist/compiled/babel/code-frame')
  return babelCodeFrameColumns(file, location, options)
}
