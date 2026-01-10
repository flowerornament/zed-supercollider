-- schelp.lua - Pandoc custom reader for SuperCollider help files
-- Usage: pandoc -f schelp.lua -t markdown input.schelp

local P, S, R, C, Ct, V, Cs, Cg, Cf, Cc, Cp, Cmt =
  lpeg.P, lpeg.S, lpeg.R, lpeg.C, lpeg.Ct, lpeg.V, lpeg.Cs,
  lpeg.Cg, lpeg.Cf, lpeg.Cc, lpeg.Cp, lpeg.Cmt

-- Helper: case-insensitive literal match
local function I(s)
  local patt = P(true)
  for i = 1, #s do
    local c = s:sub(i, i)
    patt = patt * (P(c:lower()) + P(c:upper()))
  end
  return patt
end

-- Basic patterns
local ws = S(" \t")^0
local newline = P"\r"^-1 * P"\n"
local blankline = ws * newline
local anychar = P(1)
local eof = -anychar

-- Tag ending pattern
local tag_end = P"::"

-- Escaped double colon
local escaped_colon = P"\\::" / "::"

-- Match up to tag_end, handling escapes
local function content_until_tag_end()
  return Cs((escaped_colon + (1 - tag_end))^0)
end

-- Case-insensitive tag pattern (returns capture of content after tag)
local function tag(name)
  return ws * I(name) * tag_end * ws
end

-- Tag with inline content (e.g., "code::content::")
local function inline_tag(name)
  return tag(name) * content_until_tag_end() * tag_end
end

-------------------------------------------------------------------------------
-- Header parsing
-------------------------------------------------------------------------------

local header_tags_set = {
  class = true, title = true, summary = true,
  related = true, categories = true, redirect = true
}

local function parse_header(text)
  local meta = {}
  local body_start = 1

  -- Parse header tags line by line until we hit a non-header tag
  local pos = 1
  while pos <= #text do
    local line_end = text:find("\n", pos) or #text + 1
    local line = text:sub(pos, line_end - 1)
    local trimmed = line:match("^%s*(.-)%s*$")

    -- Check if line is a header tag
    local tag_name, tag_content = trimmed:match("^([%a]+)::%s*(.*)$")
    if tag_name and header_tags_set[tag_name:lower()] then
      meta[tag_name:lower()] = tag_content:match("^%s*(.-)%s*$") or ""
      body_start = line_end + 1
    elseif trimmed == "" then
      -- Skip blank lines in header
      body_start = line_end + 1
    else
      -- Non-header content found, stop parsing header
      break
    end
    pos = line_end + 1
  end

  return meta, text:sub(body_start)
end

-------------------------------------------------------------------------------
-- Inline formatting
-------------------------------------------------------------------------------

-- Known inline tag names (for nesting detection)
local inline_tag_names = {
  code = true, teletype = true, link = true, anchor = true,
  strong = true, emphasis = true, soft = true,
  note = true, warning = true, footnote = true,
  math = true, image = true
}

local function parse_inline(text)
  local result = pandoc.Inlines{}
  local pos = 1

  while pos <= #text do
    -- Try to match inline tags
    local tag_start, tag_end_pos, tag_name = text:find("([%a]+)::", pos)

    if tag_start and tag_start == pos and inline_tag_names[tag_name:lower()] then
      local tname = tag_name:lower()

      -- Find the matching closing :: (respecting nested known tags)
      local content_start = tag_end_pos + 1
      local close_pos = nil
      local depth = 1
      local i = content_start

      while i <= #text do
        -- Check for nested known tag opening (known_tag::)
        local nested_match_start, nested_match_end, nested_name = text:find("^([%a]+)::", i)
        if nested_match_start and inline_tag_names[nested_name:lower()] then
          depth = depth + 1
          i = nested_match_end + 1
        elseif text:sub(i, i + 1) == "::" then
          -- Check if escaped
          if i == 1 or text:sub(i - 1, i - 1) ~= "\\" then
            depth = depth - 1
            if depth == 0 then
              close_pos = i
              break
            end
          end
          i = i + 2
        else
          i = i + 1
        end
      end

      if close_pos then
        local content = text:sub(content_start, close_pos - 1)
        -- Unescape
        content = content:gsub("\\::", "::")

        if tname == "code" or tname == "teletype" then
          result:insert(pandoc.Code(content))
        elseif tname == "strong" then
          result:insert(pandoc.Strong(parse_inline(content)))
        elseif tname == "emphasis" then
          result:insert(pandoc.Emph(parse_inline(content)))
        elseif tname == "soft" then
          result:insert(pandoc.Span(parse_inline(content), {class = "soft"}))
        elseif tname == "link" then
          -- Parse link: Classes/SinOsc or Classes/SinOsc#method or http://...
          local url, display = content, nil
          local hash_pos = content:find("#")
          if content:match("^https?://") then
            -- External URL
            url = content
            display = content
          elseif hash_pos then
            -- Internal link with anchor
            local path = content:sub(1, hash_pos - 1)
            local anchor = content:sub(hash_pos + 1)
            -- Extract class name for display
            local class_name = path:match("([^/]+)$") or path
            display = class_name .. "." .. anchor:gsub("^%*", "")
            url = path .. ".md#" .. anchor:gsub("^%*", ""):lower()
          else
            -- Internal link without anchor
            local class_name = content:match("([^/]+)$") or content
            display = class_name
            url = content .. ".md"
          end
          result:insert(pandoc.Link(display, url))
        elseif tname == "anchor" then
          result:insert(pandoc.Span("", {id = content}))
        elseif tname == "note" then
          result:insert(pandoc.Strong({pandoc.Str("Note: ")}))
          result:extend(parse_inline(content))
        elseif tname == "warning" then
          result:insert(pandoc.Strong({pandoc.Str("Warning: ")}))
          result:extend(parse_inline(content))
        else
          -- Unknown tag, just include content
          result:extend(parse_inline(content))
        end

        pos = close_pos + 2
      else
        -- No closing ::, treat as text
        result:insert(pandoc.Str(text:sub(pos, pos)))
        pos = pos + 1
      end
    elseif tag_start and tag_start == pos then
      -- Found word:: pattern but not a known inline tag
      -- Include the literal "word::" as text and move past it
      result:insert(pandoc.Str(tag_name .. "::"))
      pos = tag_end_pos + 1
    else
      -- Regular text until next known tag or end
      -- Find next KNOWN inline tag
      local next_tag = #text + 1
      local search_pos = pos
      while search_pos <= #text do
        local s, e, name = text:find("([%a]+)::", search_pos)
        if s then
          if inline_tag_names[name:lower()] then
            next_tag = s
            break
          else
            search_pos = e + 1
          end
        else
          break
        end
      end

      local plain_text = text:sub(pos, next_tag - 1)
      if #plain_text > 0 then
        -- Convert newlines in inline content to spaces
        plain_text = plain_text:gsub("\n", " ")
        result:insert(pandoc.Str(plain_text))
      end
      pos = next_tag
    end
  end

  return result
end

-------------------------------------------------------------------------------
-- Block parsing
-------------------------------------------------------------------------------

-- Tags that are structural sections (not block content)
local section_tags_set = {
  description = true, classmethods = true, instancemethods = true,
  examples = true, returns = true, discussion = true,
  list = true, numberedlist = true, tree = true,
  private = true
}

local function parse_blocks(text)
  local blocks = pandoc.Blocks{}
  local lines = {}
  for line in (text .. "\n"):gmatch("(.-)\n") do
    table.insert(lines, line)
  end

  local i = 1
  local current_section = nil
  local in_method = nil
  local in_argument = nil
  local method_context = nil  -- "classmethods" or "instancemethods"

  local function flush_paragraph(para_lines)
    if #para_lines > 0 then
      local para_text = table.concat(para_lines, "\n")
      para_text = para_text:match("^%s*(.-)%s*$")  -- trim
      if #para_text > 0 then
        blocks:insert(pandoc.Para(parse_inline(para_text)))
      end
    end
    return {}
  end

  local para_lines = {}

  while i <= #lines do
    local line = lines[i]
    local trimmed = line:match("^%s*(.-)%s*$")

    -- Check for standalone tag (tag::)
    local standalone_tag = trimmed:lower():match("^(%a+)::$")

    -- Handle code blocks first (before section tags)
    if standalone_tag == "code" then
      para_lines = flush_paragraph(para_lines)
      -- Collect code until closing ::
      local code_lines = {}
      i = i + 1
      while i <= #lines do
        local code_line = lines[i]
        if code_line:match("^::%s*$") then
          break
        end
        table.insert(code_lines, code_line)
        i = i + 1
      end
      local code_text = table.concat(code_lines, "\n")
      blocks:insert(pandoc.CodeBlock(code_text, {class = "supercollider"}))
      i = i + 1

    -- Handle note/warning blocks
    elseif standalone_tag == "note" or standalone_tag == "warning" then
      para_lines = flush_paragraph(para_lines)
      local block_type = standalone_tag
      local content_lines = {}
      i = i + 1
      while i <= #lines do
        local note_line = lines[i]
        if note_line:match("^::%s*$") then
          break
        end
        table.insert(content_lines, note_line)
        i = i + 1
      end
      local content_text = table.concat(content_lines, "\n")
      local prefix = block_type == "warning" and "**Warning:** " or "**Note:** "
      blocks:insert(pandoc.BlockQuote({pandoc.Para(parse_inline(prefix .. content_text))}))
      i = i + 1

    -- Handle section tags
    elseif standalone_tag and section_tags_set[standalone_tag] then
      para_lines = flush_paragraph(para_lines)

      if standalone_tag == "description" then
        blocks:insert(pandoc.Header(2, "Description"))
        current_section = "description"
      elseif standalone_tag == "classmethods" then
        blocks:insert(pandoc.Header(2, "Class Methods"))
        current_section = "classmethods"
        method_context = "classmethods"
      elseif standalone_tag == "instancemethods" then
        blocks:insert(pandoc.Header(2, "Instance Methods"))
        current_section = "instancemethods"
        method_context = "instancemethods"
      elseif standalone_tag == "examples" then
        blocks:insert(pandoc.Header(2, "Examples"))
        current_section = "examples"
      elseif standalone_tag == "returns" then
        blocks:insert(pandoc.Para(pandoc.Strong("Returns:")))
      elseif standalone_tag == "discussion" then
        blocks:insert(pandoc.Para(pandoc.Strong("Discussion:")))
      elseif standalone_tag == "private" then
        -- Skip private section content until next section
        i = i + 1
        while i <= #lines do
          local priv_line = lines[i]:match("^%s*(.-)%s*$"):lower()
          if priv_line:match("^%a+::") and section_tags_set[priv_line:match("^(%a+)::")] then
            i = i - 1  -- Back up so main loop processes this section
            break
          end
          i = i + 1
        end
      elseif standalone_tag == "list" or standalone_tag == "numberedlist" then
        -- Start collecting list items
        local list_items = {}
        i = i + 1
        while i <= #lines do
          local list_line = lines[i]
          local item_content = list_line:match("^%s*##%s*(.*)$")
          if item_content then
            table.insert(list_items, pandoc.Plain(parse_inline(item_content)))
          elseif list_line:match("^::%s*$") then
            -- End of list
            break
          elseif list_line:match("^%s*$") then
            -- Skip blank lines in list
          else
            -- Content continuation (append to last item if exists)
            if #list_items > 0 then
              local last = list_items[#list_items]
              -- Append to the last item's content
            end
          end
          i = i + 1
        end
        if standalone_tag == "numberedlist" then
          blocks:insert(pandoc.OrderedList(list_items))
        else
          blocks:insert(pandoc.BulletList(list_items))
        end
      end
      i = i + 1

    -- Check for subsection
    elseif trimmed:lower():match("^subsection::%s*(.+)$") then
      para_lines = flush_paragraph(para_lines)
      local title = trimmed:match("^[Ss][Uu][Bb][Ss][Ee][Cc][Tt][Ii][Oo][Nn]::%s*(.+)$")
      blocks:insert(pandoc.Header(3, title))
      i = i + 1

    -- Check for subsubsection
    elseif trimmed:lower():match("^subsubsection::%s*(.+)$") then
      para_lines = flush_paragraph(para_lines)
      local title = trimmed:match("^[Ss][Uu][Bb][Ss][Uu][Bb][Ss][Ee][Cc][Tt][Ii][Oo][Nn]::%s*(.+)$")
      blocks:insert(pandoc.Header(4, title))
      i = i + 1

    -- Check for method
    elseif trimmed:lower():match("^method::%s*(.+)$") then
      para_lines = flush_paragraph(para_lines)
      local methods = trimmed:match("^[Mm][Ee][Tt][Hh][Oo][Dd]::%s*(.+)$")
      local prefix = ""
      if method_context == "classmethods" then
        prefix = "*"
      elseif method_context == "instancemethods" then
        prefix = ""
      end
      -- Split on comma for multiple methods
      local method_names = {}
      for m in methods:gmatch("([^,]+)") do
        local name = m:match("^%s*(.-)%s*$")
        table.insert(method_names, prefix .. name)
      end
      local method_str = table.concat(method_names, ", ")
      blocks:insert(pandoc.Header(4, pandoc.Code(method_str)))
      in_method = methods
      i = i + 1

    -- Check for argument
    elseif trimmed:lower():match("^argument::%s*(.+)$") then
      para_lines = flush_paragraph(para_lines)
      local arg_name = trimmed:match("^[Aa][Rr][Gg][Uu][Mm][Ee][Nn][Tt]::%s*(.+)$")
      blocks:insert(pandoc.Para({pandoc.Strong({pandoc.Str(arg_name)}), pandoc.Str(":")}))
      in_argument = arg_name
      i = i + 1

    -- Blank line
    elseif trimmed == "" then
      para_lines = flush_paragraph(para_lines)
      i = i + 1

    -- Regular content line
    else
      table.insert(para_lines, line)
      i = i + 1
    end
  end

  -- Flush any remaining paragraph
  flush_paragraph(para_lines)

  return blocks
end

-------------------------------------------------------------------------------
-- Main Reader function
-------------------------------------------------------------------------------

function Reader(input, reader_options)
  local text = tostring(input)

  -- Parse header metadata
  local meta, body = parse_header(text)

  -- Build document metadata
  local doc_meta = pandoc.Meta{}

  -- Title from class:: or title::
  local title = meta["class"] or meta["title"]
  if title then
    doc_meta.title = pandoc.MetaInlines{pandoc.Str(title)}
  end

  -- Summary as subtitle
  if meta["summary"] then
    doc_meta.subtitle = pandoc.MetaInlines(parse_inline(meta["summary"]))
  end

  -- Related as list
  if meta["related"] then
    local related_items = {}
    for item in meta["related"]:gmatch("([^,]+)") do
      item = item:match("^%s*(.-)%s*$")
      table.insert(related_items, pandoc.MetaInlines(parse_inline("link::" .. item .. "::")))
    end
    doc_meta.related = pandoc.MetaList(related_items)
  end

  -- Categories
  if meta["categories"] then
    doc_meta.categories = pandoc.MetaInlines{pandoc.Str(meta["categories"])}
  end

  -- Parse body
  local blocks = parse_blocks(body)

  -- Prepend title header if we have one
  local final_blocks = pandoc.Blocks{}
  if title then
    final_blocks:insert(pandoc.Header(1, title))
    if meta["summary"] then
      final_blocks:insert(pandoc.Para(pandoc.Emph(parse_inline(meta["summary"]))))
    end
  end
  final_blocks:extend(blocks)

  return pandoc.Pandoc(final_blocks, doc_meta)
end
