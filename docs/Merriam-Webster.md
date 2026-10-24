# Merriam-Webster 的api接口说明文档

Merriam-Webster's Collegiate® Dictionary with Audio
API Product Description
America's most trusted dictionary, now more comprehensive than ever.

Access Merriam-Webster's flagship dictionary, featuring more than 250,000 authoritative definitions, 111,000 audio pronunciations, and over 3,500 idioms and phrases. Entries for the top 500 most frequently looked-up words (including culture, humility, sycophant) have been enhanced and expanded. Expert usage guidance includes 113,000 examples of words used in context, usage labels (for example, slang, informal, chiefly British), and 130 usage paragraphs.

Recently added new words and meanings include boba, cold brew, farm-to-table, heat dome, furikake, virtualization.

Data features available in this reference
Definitions
Examples
Etymologies
Synonym and Usage paragraphs
Pronunciation symbols
Audio pronunciations
Illustrations
Spelling suggestions
Formats available
JSON
Documentation
JSON Fields
Query options
Headword
Stems (inflections, variants, etc.)

## JSON
Request URL
https://www.dictionaryapi.com/api/v3/references/collegiate/json/{word}?key=your-api-key

## Query options
Headword
Stems (inflections, variants, etc.)

## Response
```
{"meta":{  
 "id":"voluminous",
 "uuid":"0d01b967-971f-4ec5-8fe0-10513d29c39b",
 "sort":"220130400",
 "src":"collegiate",
 "section":"alpha",
 "stems":[  
    "voluminous",
    "voluminously",
    "voluminousness",
    "voluminousnesses"
 ],
 "offensive":false
},
"hwi":{  
 "hw":"vo*lu*mi*nous",
 "prs":[  
    {  
       "mw":"v\u0259-\u02c8l\u00fc-m\u0259-n\u0259s",
       "sound":{  
          "audio":"volumi02",
          "ref":"c",
          "stat":"1"
       }
    }
 ]
},
"fl":"adjective",
"def":[  
 {  
    "sseq":[  
       [  
          [  
             "sense",
             {  
                "sn":"1 a",
                "dt":[  
                   [  
                      "text",
                      "{bc}having or marked by great {a_link|volume} or bulk {bc}{sx|large||} "
                   ],
                   [  
                      "vis",
                      [  
                         {  
                            "t":"long {wi}voluminous{\/wi} tresses"
                         }
                      ]
                   ]
                ],
                "sdsense":{  
                   "sd":"also",
                   "dt":[  
                      [  
                         "text",
                         "{bc}{sx|full||} "
                      ],
                      [  
                         "vis",
                         [  
                            {  
                               "t":"a {wi}voluminous{\/wi} skirt"
                            }
                         ]
                      ]
                   ]
                }
             }
          ],
          [  
             "sense",
             {  
                "sn":"b",
                "dt":[  
                   [  
                      "text",
                      "{bc}{sx|numerous||} "
                   ],
                   [  
                      "vis",
                      [  
                         {  
                            "t":"trying to keep track of {wi}voluminous{\/wi} slips of paper"
                         }
                      ]
                   ]
                ]
             }
          ]
       ],
       [  
          [  
             "sense",
             {  
                "sn":"2 a",
                "dt":[  
                   [  
                      "text",
                      "{bc}filling or capable of filling a large volume or several {a_link|volumes} "
                   ],
                   [  
                      "vis",
                      [  
                         {  
                            "t":"a {wi}voluminous{\/wi} literature on the subject"
                         }
                      ]
                   ]
                ]
             }
          ],
          [  
             "sense",
             {  
                "sn":"b",
                "dt":[  
                   [  
                      "text",
                      "{bc}writing or speaking much or at great length "
                   ],
                   [  
                      "vis",
                      [  
                         {  
                            "t":"a {wi}voluminous{\/wi} correspondent"
                         }
                      ]
                   ]
                ]
             }
          ]
       ],
       [  
          [  
             "sense",
             {  
                "sn":"3",
                "dt":[  
                   [  
                      "text",
                      "{bc}consisting of many folds, coils, or convolutions {bc}{sx|winding||}"
                   ]
                ]
             }
          ]
       ]
    ]
 }
],
"uros":[  
 {  
    "ure":"vo*lu*mi*nous*ly",
    "fl":"adverb"
 },
 {  
    "ure":"vo*lu*mi*nous*ness",
    "fl":"noun"
 }
],
"et":[  
 [  
    "text",
    "Late Latin {it}voluminosus{\/it}, from Latin {it}volumin-, volumen{\/it}"
 ]
],
"date":"1611{ds||3||}",
"ld_link":{  
 "link_hw":"voluminous",
 "link_fl":"adjective"
},
"suppl":{  
 "examples":[  
    {  
       "t":"the building\u0027s high ceilings and {it}voluminous{\/it} spaces"
    },
    {  
       "t":"a writer of {it}voluminous{\/it} output"
    }
 ],
 "ldq":{  
    "ldhw":"voluminous",
    "fl":"adjective",
    "def":[  
       {  
          "sls":[  
             "formal"
          ],
          "sseq":[  
             [  
                [  
                   "sense",
                   {  
                      "dt":[  
                         [  
                            "text",
                            "{bc}very large {bc}containing a lot of space"
                         ]
                      ]
                   }
                ]
             ],
             [  
                [  
                   "sense",
                   {  
                      "sls":[  
                         "of clothing"
                      ],
                      "dt":[  
                         [  
                            "text",
                            "{bc}using large amounts of fabric {bc}very full"
                         ]
                      ]
                   }
                ]
             ],
             [  
                [  
                   "sense",
                   {  
                      "dt":[  
                         [  
                            "text",
                            "{bc}having very many words or pages"
                         ]
                      ]
                   }
                ]
             ]
          ]
       }
    ]
 }
},
"shortdef":[  
 "having or marked by great volume or bulk : large; also : full",
 "numerous",
 "filling or capable of filling a large volume or several volumes"
]}
```

# Data Structure: Merriam-Webster's Collegiate Dictionary

2.1 Entry Metadata: meta
Entry metadata is information about the entry, as opposed to the actual content of the entry. Entry metadata is contained in meta.

Hierarchical Context

Top-level member of the dictionary entry, occurring at the very beginning of the entry.

Display Guidance

Not intended for display.

Data Model

"meta" : object containing the members:

"id" : string  unique entry identifier within a particular dictionary data set, used in cross-references pointing to the entry. It consists of the headword, and in homograph entries, an appended colon and homograph number.

"uuid" : string  universally unique identifier

"sort" : string  a 9-digit code which may be used to sort entries in the proper dictionary order if alphabetical display is needed

"src" : string  source data set for entry—ignore

"section" : string  indicates the section the entry belongs to in print, where "alpha" indicates the main alphabetical section, "biog" biographical, "geog" geographical, "idioms" idioms, and "fw&p" the foreign words & phrases section.

"stems" : array  lists all of the entry's headwords, variants, inflections, undefined entry words, and defined run-on phrases. Each stem string is a valid search term that should match this entry.

"offensive" : Boolean  true if there is a label containing "offensive" in the entry; otherwise, false.

Example

The meta section in the second homograph for "battle".

"meta":{
    "id":"battle:2",
    "uuid":"6aaba1f1-f7ca-48ce-b801-f866b41b8988",
    "sort":"020100000",
    "src":"collegiate",
    "section":"alpha",
    "stems":[
        "batteler",
        "battelers",
        "battle",
        "battle it out",
        "battled",
        "battled it out",
        "battler",
        "battlers",
        "battles",
        "battles it out",
        "battling",
        "battling it out"
    ],
    "offensive":false
}

    
2.2 Homograph: hom
Homographs are headwords with identical spellings but distinct meanings and origins. The hom member contains a homograph number used to mark and distinguish the identically-spelled headwords.

Hierarchical Context

Top-level member of dictionary entry. When hom is present, the immediately following and/or preceding entry will have an hwi/hw member with an identical value.

Display Guidance

In superscript immediately preceding the hw.

Data Model

"hom" : number

Example

Use of hom to distinguish homograph headwords in a set of four entries. Note that the homograph number is also incorporated into the meta id to ensure a unique value.

{
  "meta":{
    "id":"test:1",
    [...]
  },
  "hom":1,
  "hwi":{
    "hw":"test",
    "prs":[{
      "mw":"\u02c8test",
      "sound":{"audio":"test0001","ref":"c","stat":"1"}
    }]
  },
  "fl":"noun",
  [...]
}, {
  "meta":{
    "id":"test:2",
    [...]
  },
  "hom":2,
  "hwi":{"hw":"test"},
  "fl":"verb",
  [...]
}, {
  "meta":{
    "id":"test:3",
    [...]
  },
  "hom":3,
  "hwi":{"hw":"test"},
  "fl":"adjective",
  [...]
}, {
  "meta":{
    "id":"test:4",
    [...]
  },
  "hom":4,
  "hwi":{"hw":"test"},
  "fl":"noun",
  [...]
}
    
XML Equivalent

Note the homograph number is stored in an {h,#} curly-brace sequence in the headword tag.

<hw>{h,1}test</hw>
2.3 Headword Information: hwi
The headword is the word being defined or translated in a dictionary entry. Key headword information is grouped in an hwi object. This includes the headword itself in the hw member, and may include one or more pronunciations in prs.

Hierarchical Context

Top-level member of the dictionary entry.

Display Guidance

The headword is prominently highlighted to the user; this is typically achieved through the use of bold, large point size, distinctive font, etc.

Data Model

"hwi" : object with the following members:

"hw" : string  headword (required)

optional prs

Example

An example of hwi in the first homograph of "test".

"hom":1,
"hwi":{
  "hw":"test",
  "prs":[{
    "mw":"\u02c8test",
    "sound":{"audio":"test0001","ref":"c","stat":"1"}
  }]
}
    
    
2.4 Alternate Headwords: ahws
An alternate headword is a regional or less common spelling of a headword. A set of one or more alternate headwords is contained in an ahws.

Hierarchical Context

Top-level member of the dictionary entry.

Display Guidance

See hwi for handling of hw.

Data Model

"ahws" : array of one or more alternate headword objects, each of which may contain the members:

"hw" : string  headword (required)

optional prs, psl

Example

The ahws and hwi for "agonise". Note the first hw in an entry will always be in an hwi.

"hwi":{"hw":"ag*o*nise"},
"ahws":[
  {
    "hw":"agonised"
  },{
    "hw":"agonising"
  }
]

    
2.5 Variants: vrs
A variant is a different spelling or styling of a headword, defined run-on phrase, or undefined entry word. A set of one or more variants is contained in a vrs.

Hierarchical Context

Occurs as top-level member of dictionary entry and in dros, ri, sdsense, sen, sense, uros.

Display Guidance

A set of variants might be displayed inline with the headword, or in its own block with a lead-in heading such as "variants:".

Each va is typically displayed in bold, less prominently than an hw.

A vl is typically displayed in italics.

Data Model

"vrs" : array of one or more variant objects, each of which may contain the members:

"va" : string	variant (required)
"vl" : string	variant label, such as “or” (optional)
prs, spl	(optional)
Example

The vrs and hwi for "kabbalah".

"hwi":{
  "hw":"kab*ba*lah",
  "prs":[
    {
      "mw":"k\u0259-\u02c8b\u00e4-l\u0259",
      "sound":{"audio":"cabala02","ref":"c","stat":"1"}
    },{
      "mw":"\u02c8ka-b\u0259-l\u0259",
      "sound":{"audio":"cabala01","ref":"c","stat":"1"}
    }
  ]
},"vrs":[
  {
    "vl":"or less commonly",
    "va":"kab*ba*la"
  },{
    "vl":"or",
    "va":"ka*ba*la"
  },{
    "vl":"or",
    "va":"ca*ba*la"
  },{
    "vl":"or",
    "va":"cab*ba*la"
  },{
    "vl":"or",
    "va":"cab*ba*lah"
  }
]
    
    
2.6 Pronunciations: prs
A pronunciation specifies how a written word is pronounced aloud. A set of pronunciations is encoded in a prs array. Each element represents a distinct pronunciation of a headword or other term.

Hierarchical Context

Occurs in ahws, cats, dros, hwi, ins, ri, sdsense, sen, sense, uros, vrs

Display Guidance

The entire set of pronunciations is typically surrounded by backslash characters.

If there are multiple pronunciation objects and an object contains a punctuation member pun, use its contents plus a space to separate the two objects (for example, "\pronunciation 1; pronunciation 2" where pun contains ";").

If pun is not present, use a comma and space to separate the two objects (for example, "\pronunciation 1, pronunciation 2\").

The l and l2 pronunciation labels are typically displayed in italics.

Data Model

"prs" : array of one or more pronunciation objects, each of which may contain the following members:

"mw" : string	written pronunciation in Merriam-Webster format
"l" : string	pronunciation label before pronunciation
"l2" : string	pronunciation label after pronunciation
"pun" : string	punctuation to separate pronunciation objects
"sound" : object	audio playback information: the audio member contains the base filename for audio playback; the ref and stat members can be ignored.
An audio reference URL should be in the following form:

https://media.merriam-webster.com/audio/prons/[language_code]/[country_code]/[format]/[subdirectory]/[base filename].[format]
Where the values in brackets are determined as follows:

[language_code] - 2 letter ISO language code.

en for all API references except the Spanish-English Dictionary.
es for entries with a "lang": "es" metadata value in the Spanish-English Dictionary
[country_code] - 2 letter ISO country code.

us for all API references except the Spanish-English Dictionary.
me for entries with a "lang": "es" metadata value in the Spanish-English Dictionary
[format] - One of 3 audio formats supported for all audio values:

mp3
wav
ogg
[subdirectory] is determined as follows:

if audio begins with "bix", the subdirectory should be "bix",
if audio begins with "gg", the subdirectory should be "gg",
if audio begins with a number or punctuation (eg, "_"), the subdirectory should be "number",
otherwise, the subdirectory is equal to the first letter of audio.
[base filename] - The name contained in audio.

As examples, the URL for the object {"audio":"3d000001","ref":"c","stat":"1"} in the Collegiate entry "3-D" would be: https://media.merriam-webster.com/audio/prons/en/us/mp3/number/3d000001.mp3, while the URL for the object {"audio": "hola001sp"} in the Spanish-English entry "hola" would be: https://media.merriam-webster.com/audio/prons/es/me/mp3/h/hola001sp.mp3.

Example

A prs with multiple pronunciations in the entry "pajama" of collegiate reference for example. If you were to generate an audio URL for the first sound object below, it would be: https://media.merriam-webster.com/audio/prons/en/us/mp3/p/pajama02.mp3

"hwi":{
  "hw":"pa*ja*ma",
  "prs":[
    {
      "mw":"p\u0259-\u02c8j\u00e4-m\u0259",
      "sound":{
        "audio":"pajama02",
        "ref":"c",
        "stat":"1"
      }
    },{
      "mw":"-\u02c8ja-",
      "sound":{
        "audio":"pajama01",
        "ref":"c",
        "stat":"1"
      }
    }
  ]
}
    

    
2.7 Labels
A label provides a brief note on the grammatical function, subject area, register, regional usage, or capitalization of a headword, whether generally or in a particular sense.

2.7.1 Functional Label: fl
The functional label describes the grammatical function of a headword or undefined entry word, such as "noun" or "adjective".

Hierarchical Context

Occurs as a top-level member of the dictionary entry, where it describes the hw in the preceding hwi. Also occurs within uros, where it describes the preceding ure.

Display Guidance

Typically rendered in italics

Data Model

"fl" : string

Example

Functional label for the second homograph of "traffic". This makes it clear to the user that the verb "traffic" is being defined.

"hom":2,
"hwi":{"hw":"traffic"},
"fl":"verb"
    
    
2.7.2 General Labels: lbs
General labels provide information such as whether a headword is typically capitalized, used as an attributive noun, etc. A set of one or more such labels is contained in an lbs.

Hierarchical Context

Occurs as top-level member of dictionary entry and in dros, sdsense, sen, sense, or uros.

Display Guidance

Typically rendered in italics. If there is a more than one element in the array, separate them with a comma and space.

Data Model

"lbs" : array of one or more elements

Example

An lbs in the entry for "deco".

"lbs":[
  "often capitalized",
  "often attributive"
],"def":[
  {
    "sseq":[
      [
        [
          "sense",
          {
            "dt":[["text","{bc}{sx|art deco||}"]]
          }
        ]
      ]
    ]
  }
]
    

    
2.7.3 Subject/Status Labels: sls
A subject/status label describes the subject area (eg, "computing") or regional/usage status (eg, "British", "formal", "slang") of a headword or a particular sense of a headword. A set of one or more subject/status labels is contained in an sls.

Hierarchical Context

Occurs as top-level member of dictionary entry and in def, dros, sdsense, sen, sense, or uros.

Display Guidance

Typically rendered in italics. If there is a more than one element in the array, separate with a comma and space.

Data Model

"sls" : array of one or more elements

Example

A pair of subject/status labels in sense 3b of "ginger".

[
  "sense",
  {
    "sn":"b",
    "sls":[
      "chiefly British",
      "sometimes offensive"
    ],
    "dt":[
      [
        "text",
        "{bc}a person with red hair {bc}{sx|redhead||1} "
      ],[
        "vis",
        [
          {
            "t":"The Breda Redhead Days festival\u2014which grew out of
            a photo shoot by Dutch artist Bart Rouwenhorst\u2014now
            attracts five or six thousand {wi}gingers{\/wi} from around
            the world.",
            "aq":{"auth":"Bruce Ingram"}
          }
        ]
      ]
    ]
  }
]
    
    
2.7.4 Parenthesized Subject/Status Label: psl
A parenthesized subject/status label describes the subject area or regional/usage status (eg, "British") of a headword or other defined term, and is displayed in parentheses. The parenthesized subject/status label is contained in a psl.

Hierarchical Context

Occurs in ahws, cats, dros, hwi, vi, or uros.

Display Guidance

Display within parentheses and in italics.

Data Model

"psl" : string

Example

An example of psl in the entry "meow".

"hwi":{
  "hw":"me*ow",
  "prs":[
    {
      "mw":"m\u0113-\u02c8au\u0307",
      "sound":{"audio":"meow0001","ref":"c","stat":"1"}
    }
  ],
  "psl":"chiefly US"
},
"vrs":[
  {
    "vl":"or British",
    "va":"mi*aow"
  }
],
"fl":"noun"
    

    
2.7.5 Sense-Specific Inflection Plural Label: spl
This label provides information on the grammatical number (eg, singular, plural) of an inflection in a particular sense. A sense-specific inflection plural label is contained in an spl.

Hierarchical Context

Occurs in ins.

Display Guidance

Typically rendered in italics.

Data Model

"spl" : string

Example

In the entry for "acoustics", sense 1 contains an spl noting that in this sense the plural noun is grammatically singular (eg, "Acoustics is the science of sound").

"fl":"plural noun",
"def":[
  {
    "sseq":[
      [
        [
          "sense",
          {
            "sn":"1",
            "ins":[
              {"spl":"singular in construction"}
            ],
            "dt":[
              ["text","{bc}a science that deals with the production,
              control, transmission, reception, and effects of sound"]
            ]
          }
        ]
      ],[
        [
          "sense",
          {
            "sn":"2",
            "ins":[
              {
                "il":"also",
                "if":"acoustic"
              }
            ],
            "dt":[
              ["text","{bc}the qualities that determine the ability of an
              enclosure (such as an auditorium) to reflect sound waves in
              such a way as to produce distinct hearing"]
            ]
          }
        ]
      ]
    ]
  }
]
    
    
2.7.6 Sense-Specific Grammatical Label: sgram
This label notes whether a particular sense of a verb is transitive (T) or intransitive (I). The sense-specific grammatical label is contained in an sgram.

Hierarchical Context

Occurs in sdsense, sen, sense.

Display Guidance

Typically displayed within square brackets and in italics.

Data Model

"sgram" : string

Example

Use of sgram in two senses of "reboot".

[
  "sense",
  {
    "sn":"1 a",
    "sgram":"T\/I",
    "dt":[
      ["text","{bc}to shut down and restart (a computer or program) "],
      [
        "vis",
        [
          {
            "t":"\u2026 the annoyance of having to {wi}reboot{\/wi} the
            computer to switch operating systems \u2026",
            "aq":{"auth":"Robert Weston"}
          },
          {
            "t":"If anything ever happens to the original drive, you can
            {wi}reboot{\/wi} using the cloned drive and be up and running
            in minutes.",
            "aq":{"auth":"Dan Frakes"}
          }
        ]
      ]
    ]
  }
],[
  "sense",
  {
    "sn":"b",
    "sgram":"I",
    "dt":[
      ["text","{bc}to start up again after closing or shutting down {bc}to
      boot up again "],
      [
        "vis",
        [
          {
            "t":"waiting for a computer\/program to {wi}reboot{\/wi}"
          }
        ]
      ]
    ]
  }
]
    
    
2.8 Inflections: ins
Inflection is the change of form that words undergo in different grammatical contexts, such as tense or number. A set of one or more inflections is contained in an ins.

Hierarchical Context

Occurs as top-level member of dictionary entry and in sdsense, sen, sense, or uros.

Display Guidance

If both if and ifc are present, only one should be displayed to the user. The ifc has traditionally been used in print publications. Both are typically displayed in bold.

The il inflection label is typically displayed in italics, and should be followed by a space.

Inflection objects should be separated by a semicolon and space unless the second object of a pair contains il, in which case they should be separated by a space.

Data Model

"ins" : array of one or more inflection objects, each of which may contain the members:

"if" : string	inflection: a fully spelled-out inflection
"ifc" : string	inflection cutback: an inflection ending (eg, "-ing")
"il" : string	inflection label, such as “also”, “plural”, “or”
prs, spl	(optional)
Example

Inflections in the second homograph of "tassel".

"ins":[
  {
    "ifc":"-seled",
    "if":"tas*seled"
  },{
    "il":"or",
    "ifc":"-selled",
    "if":"tas*selled"
  },{
    "ifc":"-sel*ing",
    "if":"tas*sel*ing"
  },{
    "il":"or",
    "ifc":"-sel*ling",
    "if":"tas*sel*ling",
    "prs":[
      {
        "mw":"\u02c8ta-s(\u0259-)li\u014b",
        "sound":{"audio":"tassel02","ref":"c","stat":"1"}
      }
    ]
  }
]
    

    
2.9 Cognate Cross-References: cxs
When a headword is a less common spelling of another word with the same meaning, there will be a cognate cross-reference pointing to the headword with the more common spelling. A set of cognate cross-references is contained in a cxs.

Hierarchical Context

Top-level member of dictionary entry.

Display Guidance

The cxt cross-reference target is typically displayed in smallcaps. If cxr is present, do not display but use in forming cross-reference hyperlink as described below.

The cxl label is typically displayed in italics and should be followed by a space.

A cxn is typically displayed in normal font and should be preceded by a space.

If the cxs array has more than one element, separate them by a comma and space.

Data Model

"cxs" : array of one or more elements:

"cxl" : string  cognate cross-reference label

"cxtis" : array  of one or more cognate cross-reference targets, each of which may contain:

"cxl" : string  cognate cross-reference label

"cxr" : string  when present, use as cross-reference target ID for immediately preceding cxt

"cxt" : string  provides hyperlink text in all cases, and cross-reference target ID when no immediately following cxr

"cxn" : string  sense number of cross-reference target

Example

A cxs in the first homograph of "baloney" with a cxt pointing to the more common spelling "bologna". Although the cross-reference is provided, note in this case the definition of "bologna" is included for ease of reference.

"hom":1,
"hwi":{
  "hw":"ba*lo*ney",
  "prs":[
    {
      "mw":"b\u0259-\u02c8l\u014d-n\u0113",
      "sound":{"audio":"bologn01","ref":"c","stat":"1"}
    }
  ]
},
"fl":"noun",
"cxs":[
  {
    "cxl":"less common spelling of",
    "cxtis":[
      {"cxt":"bologna"}
    ]
  }
],
"def":[
  {
    "sseq":[
      [
        [
          "sense",
          {
            "dt":[
              ["text","{bc}a large smoked sausage of beef, veal, and pork"]
            ],
            "sdsense":{
              "sd":"also",
              "dt":[
                ["text","{bc}a sausage made (as of turkey) to resemble bologna"]
              ]
            }
          }
        ]
      ]
    ]
  }
]
    
    
2.10 Sense Organization
The sense is a key organizational unit of the entry, and gathers together all content relevant to a particular meaning of a headword. Senses are presented in a numbered series, with further divisions into subsenses identified by lowercase letters and parenthesized numbers. Sense sequences are organized by part of speech for verb entries: if a verb can be both transitive and intransitive, there will be two verb dividers, one marking the sense sequence for the transitive verb and the other the sense sequence for the intransitive verb.

The main definition section def encompasses all of the sense sequences and verb dividers vd for a headword. A sense sequence sseq groups together the numbered senses (sense and sen) defining particular meanings of the headword. Finally, more complex sense structures are represented by pseq, sdsense, and bs.

2.10.1 Definition Section: def
The definition section groups together all the sense sequences and verb dividers for a headword or defined run-on phrase.

Hierarchical Context

Occurs as top-level member of dictionary entry and in dros.

Display Guidance

Typically displayed in a new paragraph.

Data Model

array of one or more objects

Example

The def definition section for the entry "backflip".

"def":[
  {
    "sseq":[
      [
        [
          "sense",
          {
            "dt":[["text","{bc}a {a_link|backward} somersault especially
              in the air"]]
          }
        ]
      ]
    ]
  }
]

    
2.10.2 Verb Divider: vd
The verb divider acts as a functional label in verb entries, introducing the separate sense sequences for transitive and intransitive meanings of the verb.

Hierarchical Context

Occurs in def, preceding an sls (optional) and sseq (required)

Display Guidance

Typically rendered in italics

Data Model

"vd" : string

Example

Two vd verb dividers in the second homograph of "boat".

"def":[
  {
    "vd":"transitive verb",
    "sseq":[
      [
        [
          "sense",
          {
            "dt":[["text","{bc}to place in or bring into a boat"]]
          }
        ]
      ]
    ]
  },{
    "vd":"intransitive verb",
    "sseq":[
      [
        [
          "sense",
          {
            "dt":[["text","{bc}to go by boat"]]
          }
        ]
      ]
    ]
  }
]
    
2.10.3 Sense Sequence: sseq
The sense sequence contains a series of senses and subsenses, ordered by sense numbers beginning at Arabic numeral "1". The sense sequence is contained in an sseq.

Hierarchical Context

Occurs in def

Display Guidance

See sense

Data Model

"sseq" : array

Example

An sseq from the second homograph of "hop".

"sseq":[
  [
    [
      "sense",
      {
        "sn":"1 a",
        "dt":[["text","{bc}a short brisk leap especially on one leg"]]
      }
    ],[
      "sense",
      {
        "sn":"b",
        "dt":[
          ["text","{bc}{sx|bounce||}, {sx|rebound||} "],
          [
            "vis",
            [{"t":"shortstop scooped it up on the first {wi}hop{\/wi}"}]
          ]
        ]
      }
    ]
  ],[
    [
      "sense",
      {
        "sn":"2",
        "dt":[["text","{bc}{sx|dance||3}"]]
      }
    ]
  ],[
    [
      "sense",
      {
        "sn":"3 a",
        "dt":[["text","{bc}a flight in an aircraft"]]
      }
    ],[
      "sense",
      {
        "sn":"b",
        "dt":[["text","{bc}a short trip"]]
      }
    ]
  ]
]
    
    
2.10.4 Sense: sense
The sense gathers together all content relevant to a particular meaning of a headword or defined run-on phrase.

Hierarchical Context

Occurs in an sseq, pseq, or bs

Display Guidance

A typical implementation might generate a newline at a top-level numbered sense, while keeping further nested subsenses inline.

Data Model

object or array consisting of one dt (required) and zero or more et, ins, lbs, prs, sdsense, sgram, sls, sn, or vrs

Example

An array containing two senses in "test".

[
  [
    "sense",
    {
      "sn":"1 a",
      "dt":[
        [
          "text",
          "{bc}to undergo a test"
        ]
      ]
    }
  ],[
    "sense",
    {
      "sn":"b",
      "dt":[
        [
          "text",
          "{bc}to be assigned a standing or evaluation on the
           basis of {a_link|tests} "
        ],[
          "vis",
          [
            {"t":"{wi}tested{\/wi} positive for cocaine"},
            {"t":"the cake {wi}tested{\/wi} done"}
          ]
        ]
      ]
    }
  ]
]
    
    
2.10.5 Sense Number: sn
The sense number identifies a sense or subsense within the entry. A sense number sn may contain bold Arabic numerals, bold lowercase letters, or parenthesized Arabic numerals.

Hierarchical Context

Occurs within a sense or sen

Display Guidance

The sense number is typically displayed in bold.

Data Model

"sn" : string

Example

A sequence of three senses and sense numbers in the entry "tertiary".

[
  "sense",
  {
    "sn":"1 a",
    "dt":[["text","{bc}of third rank, importance, or value"]]
  }
],[
  "sense",
  {
    "sn":"b",
    "sls":["chiefly British"],
    "dt":[["text","{bc}of, relating to, or being higher education"]]
  }
],[
  "sense",
  {
    "sn":"c",
    "dt":[["text","{bc}of, relating to, or constituting the third strongest
     of the three or four degrees of stress recognized by most linguists (such
     as the stress of the third syllable of {it}basketball team{\/it})"]]
  }
]
    
    
2.10.6 Defining Text: dt
The defining text is the text of the definition or translation for a particular sense. Defining text is contained in dt.

Hierarchical Context

Occurs in a sense or sdsense

Display Guidance

Inline in normal font

Data Model

"dt" : array consisting of one or more elements:

["text", string] where string contains the definition content (required)

optional bnw, ca, ri, snote, uns, or vis elements

Example

A definition of "test" containing both text and vis elements.

"dt":[
  [
    "text",
    "{bc}the procedure of submitting a statement to such conditions
     or operations as will lead to its proof or disproof or to its
     acceptance or rejection "
  ],[
    "vis",
    [
      {"t":"a {wi}test{\/wi} of a statistical hypothesis"}
    ]
  ]
]
    
    
2.10.7 Divided Sense: sdsense
A sense may be divided into two parts to show a particular relationship between two closely related meanings. The second part of the sense is contained in an sdsense, consisting of an sense divider sd along with a second dt.

Hierarchical Context

Occurs within a sense, where it is always preceded by dt.

Display Guidance

The sdsense should be inline with the preceding dt. The sd is displayed in italics, preceded by a semicolon and space, and followed by a space.

Data Model

"sdsense" : object with the following members:

"sd" : string	sense divider (required)
et, ins, lbs, prs, sgram, sls, vrs	(optional)
dt	definition text (required)
Example

An example of a divided sense from the entry "abiogenesis".

"def":[
  {
    "sseq":[
      [
        [
          "bs",
          {
            "sense":{
              "dt":[["text","{bc}the origin of life from nonliving matter"]],
              "sdsense":{
                "sd":"specifically",
                "dt":[
                  ["text","{bc}a theory in the evolution of early life on earth
                    {bc}organic molecules and subsequent simple life forms first
                    originated from inorganic substances "],
                  [etc...]
                ]
              }
            }
          }
        ]
      ]
    ]
  }
]

    
2.10.8 Truncated Sense: sen
A truncated sense is a sense without a definition, and is used almost exclusively when two sense numbers are separated by a non-definition element. For instance, if a pronunciation occurs in between a main sense number and a subsense containing a definition, the truncated sense sen consists of the main sense number and the pronunciation.

Hierarchical Context

Occurs as an element in an sseq array.

Display Guidance

Handle the same as sense.

Data Model

object or array containing at least one of the set et, ins, lbs, prs, sgram, sls, sn, vrs

Example

Example of an sen truncated sense in the entry for "tab", followed by two full sense elements containing definition text dt. The sen element is considered truncated because it lacks a definition, but is otherwise similar to sense.

[
  [
    "sen",
    {
      "sn":"4",
      "et":[
        ["text","by shortening"]
      ]
    }
  ],[
    "sense",
    {
      "sn":"a",
      "dt":[
        ["text","{bc}{sx|tablet||}"]
      ]
    }
  ],[
    "sense",
    {
      "sn":"b",
      "dt":[
        ["text","{bc}{sx|tabloid||}"]
      ]
    }
  ]
]
    
    
2.10.9 Binding Substitute: bs
The binding substitute is a broad, general sense introducing a series of senses that give more contextual and specific meanings.

Hierarchical Context

Occurs as an element in an sseq or pseq array, where it is followed by one or more sense elements.

Display Guidance

Inline in normal font

Data Model

array of the form ["bs", {sense}]

Example

An example of a bs in sense 2 of "feline". Please also see the example of a bs within a pseq below.

[
  [
    "bs",
    {
      "sense":{
        "sn":"2",
        "dt":[["text","{bc}resembling a cat: such as"]]
      }
    }
  ],[
    "sense",{
      "sn":"a",
      "dt":[["text","{bc}sleekly graceful"]]
    }
  ],[
    "sense",
    {
      "sn":"b",
      "dt":[["text","{bc}{sx|sly||}, {sx|treacherous||}"]]
    }
  ],[
    "sense",
    {
      "sn":"c",
      "dt":[["text","{bc}{sx|stealthy||}"]]
    }
  ]
]
    
    
2.10.10 Parenthesized Sense Sequence: pseq
The parenthesized sense sequence groups together senses whose sense numbers form a sequence of parenthesized numbers.

Hierarchical Context

Occurs as an element in an sseq array.

Display Guidance

If you are generating sense numbers for sense elements in a pseq sequence, put parentheses around the number. For example, the second sense in a sequence should have "(2)" as its sense number.

If you are instead using the sn to display the sense number, it will already contain the parentheses.

Data Model

array consisting of one or more sense elements and an optional bs element.

Example

In this example from "tab", the pseq contains a sequence of three elements: bs (binding substitute), sense, and sense. The sense numbers generated at each sense should be in parentheses.

[
  "pseq",
  [
    [
      "bs",
      {
        "sense":{
          "sn":"1 a",
          "dt":[
            ["text","{bc}a short projecting device: such as"]
          ]
        }
      }
    ],[
      "sense",
      {
        "sn":"(1)",
        "dt":[
          ["text","{bc}a small flap or loop by which something may be grasped
          or pulled"]
        ]
      }
    ],[
      "sense",
      {
        "sn":"(2)",
        "dt":[
          ["text","{bc}a projection from a card used as an aid in filing"]
        ]
      }
    ]
  ]
]
    
    
2.11 Verbal Illustrations: vis
A verbal illustration is an example sentence illustrating how a word is used in context. It may either be a made-up sentence or a quotation from a cited source. A set of verbal illustrations is contained in a vis.

Hierarchical Context

Occurs in dt, et_snote, pt, snote, uns, utxt.

Display Guidance

A verbal illustration is typically set off from surrounding text (as by surrounding the entire illustration in angle brackets).

Data Model

Array of the form ["vis", [{object}]], where each object may contain the members:

"t" : string  text of verbal illustration

optional aq (used when verbal illustration is an author quotation as opposed to made-up example)

Example

Verbal illustrations vis in the second homograph of "above".

[
  "sense",
  {
    "sn":"2 a",
    "dt":[
      ["text","{bc}superior to (as in rank, quality, or degree) "],
      [
        "vis",
        [
          {"t":"A sergeant is {wi}above{\/wi} a corporal."}
        ]
      ]
    ]
  }
],[
  "sense",
  {
    "sn":"b",
    "dt":[
      ["text","{bc}out of reach of "],
      [
        "vis",
        [
          {"t":"{wi}above{\/wi} suspicion"}
        ]
      ]
    ]
  }
],[
  "sense",
  {
    "sn":"c",
    "dt":[
      ["text","{bc}in preference to "],
      [
        "vis",
        [
          {"t":"puts his child\u0027s needs {wi}above{\/wi} his
          own needs"}
        ]
      ]
    ]
  }
],[
  "sense",
  {
    "sn":"d",
    "dt":[
      ["text","{bc}too proud or honorable to stoop to "],
      [
        "vis",
        [
          {"t":"not {wi}above{\/wi} taking undue credit"}
        ]
      ]
    ]
  }
]
    
    
2.12 Attribution of Quote: aq
Direct quotes are used in both verbal illustrations and end-of-entry quotation sections. Information about the attribution (the author, publication, date, etc.) of a particular quote is contained in an aq.

Hierarchical Context

Occurs in quotes, vis.

Display Guidance

The aq is typically preceded by an em-dash.

Each instance of auth, source, aqdate, should be followed by a comma and space.

Data Model

"aq" : object, which may contain the members:

"auth" : string  name of author

"source" : string  source work for quote

"aqdate" : string  publication date of quote

"subsource" : object  further detail on quote source (eg, name of larger work in which an essay is found); may contain the members source and aqdate (described above)

Example

Use of aq in the entry for "alliteration".

"quotes":[
  {
    "t":"More specifically, how are actual events deformed by the application
    to them of metaphor, rhetorical comparison, prose rhythm, assonance,
    {qword}alliteration{\/qword}, allusion, and sentence structures and
    connectives implying clear causality?",
    "aq":{
      "auth":"Paul Fussel",
      "source":"{it}The Great War and Modern Memory{\/it}",
      "aqdate":"1975"
    }
  },{
    "t":"As far as sound repetition goes, I don\u0027t have any principles.
    I try to stay away from heavy {qword}alliteration{\/qword} and other
    pyrotechnics because I think they detract from the sense of the poem
    and blur the imagery.",
    "aq":{
      "auth":"Maxine Kumin",
      "source":"\u0022A Questionnaire\u0022",
      "aqdate":"1977",
      "subsource":{
        "source":"in {it}To Make a Prairie{\/it}",
        "aqdate":"1979"
      }
    }
  }
]

    
2.13 Run-In: ri
A run-in entry word is a defined word that occurs in the running text of an entry. The run-in ri groups together one or more run-in entry words rie and any accompanying pronunciations or variants. Run-ins occur most frequently in geographical entries.

Hierarchical Context

Occurs in dt, et_snote, snote, uns.

Display Guidance

Inline in normal font, except for rie which is typically displayed in bold.

Data Model

Array of the form ["ri", [[array]]], where each element may contain:

"riw", object  run-in wrap which may contain:

{"rie" : string}  run-in entry word

optional prs

"text", string  intervening text

optional vrs

Example

A run-in ri with two run-in entry words rie in the geographical entry "Abaco".

"def":[
  {
    "sseq":[
      [
        [
          "sense",
          {
            "dt":[
              ["text","two islands of the Bahamas ("],
              [
                "ri",
                [
                  [
                    "riw",
                    {"rie":"Great Abaco"}
                  ],[
                    "text"," and "
                  ],[
                    "riw",
                    {"rie":"Little Abaco"}
                  ]
                ]
              ],[
                "text",
                " ) north of New Providence Island {it}area{\/it}
                776 square miles (2018 square kilometers),
                {it}population{\/it} 13,170"
              ]
            ]
          }
        ]
      ]
    ]
  }
]
    
    
2.14 Biographical Name Wrap: bnw
A biographical name wrap groups together personal name, surname, and alternate name information within a biographical entry. The biographical name wrap is contained in a bnw.

Hierarchical Context

Occurs in dt.

Display Guidance

A bnw is displayed inline and followed by a comma and space.

A pname or sname is typically displayed in normal font.

An altname is typically displayed in italics.

Data Model

Array of the form "bnw", object, which may contain the members:

"pname" : string  personal or first name

"sname" : string  surname

"altname" : string  alternate name such as pen name, nickname, title, etc.

Example

A biographical name wrap bnw in the entry for "Dodgson".

"def":[
  {
    "sseq":[
      [
        [
          "sense",
          {
            "dt":[
              [
                "bnw",
                {
                  "pname":"Charles Lut*widge",
                  "prs":[
                    {
                      "mw":"\u02c8l\u0259t-wij",
                      "sound":{"audio":"bixdod04","ref":"c","stat":"1"}
                    }
                  ]
                }
              ],[
                "text",
                "1832\u20131898 pseudonym"
              ],[
                "bnw",
                {
                  "altname":"Lewis Car*roll",
                  "prs":[
                    {
                      "mw":"\u02c8ker-\u0259l",
                      "sound":{"audio":"bixdod05","ref":"c","stat":"1"}
                    },{
                      "mw":"\u02c8ka-r\u0259l"
                    }
                  ]
                }
              ],[
                "text",
                " English mathematician and writer"
              ]
            ]
          }
        ]
      ]
    ]
  }
]
    

2.15 Called-Also Note: ca
A called-also note lists other names a headword is called in a given sense. The called-also note is contained in a ca.

Hierarchical Context

Occurs in dt.

Display Guidance

A ca is preceded by an em-dash and a space.

The intro is typically displayed in normal font, and the cat in italics.

The pn is displayed in parentheses, followed by a space.

Note that a ca is typically found at an entry providing detailed definition content, and hence a hyperlink is not generated for the cat target. If the cat target is entered separately in the dictionary, that entry will typically have a cross-reference back to the detailed entry where the ca occurs.

Note, however, that where a catref is present, it may be used to generate a hyperlink if desired.

Data Model

Array of the form "ca", object, which may contain the members:

"intro" : string  contains introductory text "called also"

"cats" : array  of one or more called-also targets, each of which may contain:

"cat" : string  called-also target text

"catref" : string  called-also reference containing target ID

"pn" : string  parenthesized number

optional prs, psl

Example

A ca called-also note in the entry "abaca".

[
  "sense",
  {
    "sn":"1",
    "dt":[
      [
        "text",
        "{bc}a strong fiber obtained from the leafstalk of a banana
        ({it}Musa textilis{\/it}) native to the Philippines "
      ],[
        "ca",
        {
          "intro":"called also",
          "cats":[
            {"cat":"Manila hemp"}
          ]
        }
      ]
    ]
  }
]
    
    
2.16 Supplemental Information Note: snote
This note provides explanatory or historical information that supplements the definition text. The supplemental information note is contained in an snote.

Hierarchical Context

Occurs in dt.

Display Guidance

Typically displayed after a newline and in normal font. May be preceded with introductory text such as "Note: ".

Data Model

Array of the form ["snote", [array]] containing the elements:

Required ["t", string] where string contains the supplemental information note text.

Optional ri and vis elements.

Example

An snote in the entry "manatee".

"def":[
  {
    "sseq":[
      [
        [
          "sense",
          {
            "dt":[
              [
                "text",
                "{bc}any of a genus ({it}Trichechus{\/it} of the family
                Trichechidae) of large, herbivorous, aquatic mammals
                that inhabit warm coastal and inland waters of the
                southeastern U.S., West Indies, northern South America,
                and West Africa and have a rounded body, a small head
                with a squarish snout, paddle-shaped flippers usually
                with vestigial nails, and a flattened, rounded tail
                used for propulsion "
              ],[
                "snote",
                [
                  [
                    "t",
                    "Manatees are {d_link|sirenians|sirenian} related to and
                    resembling the {d_link|dugong|dugong} but differing most
                    notably in the shape of the tail."
                  ],[
                    "vis",
                    [
                      {
                        "t":"An aquatic relative of the elephant,
                        {wi}manatees{\/wi} grow up to nine feet long
                        and can weigh 1,000 pounds.",
                        "aq":{"auth":"Felicity Barringer"}
                      }
                    ]
                  ]
                ]
              ]
            ]
          }
        ]
      ]
    ]
  }
]
    

    
2.17 Usage Notes: uns
Provide usage information on a headword, defined run-on phrase, or undefined entry word. A series of one or more usage notes is contained in an uns.

Hierarchical Context

Occurs in dt, utxt.

Display Guidance

An uns is typically displayed inline in normal font, preceded by a space and an em-dash.

Data Model

Array of the form ["uns", [[array]]], where each individual usage note array may contain the elements:

Required ["text", string] where string contains the usage note text.

Optional ri and vis elements.

Example

An example of uns in the entry for "abeyance".

[
  "sense",
  {
    "sn":"1",
    "dt":[
      [
        "text",
        "{bc}a state of temporary inactivity {bc}{sx|suspension||} "
      ],[
        "uns",
        [
          [
            [
              "text",
              "used chiefly in the phrase {phrase}in abeyance{\/phrase} "
            ],[
              "vis",
              [
                {
                  "t":"\u2026 new contracts on all but one existing mine \u2026
                  are {wi}in abeyance{\/wi} pending the outcome of a government
                  inquiry to be carried out into Australia\u0027s role in the
                  nuclear fuel cycle.",
                  "aq":{"auth":"Vimala Sarma"}
                },{
                  "t":"a plan that is currently being held {wi}in abeyance{\/wi}"
                }
              ]
            ]
          ]
        ]
      ]
    ]
  }
]
    
    
2.18 Undefined Run-Ons: uros
An undefined entry word is derived from or related to the headword, carries a functional label and possibly other information, but does not have any definitions. A set of undefined run-ons uros can follow (or "run on" from) the entry's main def definition section, with each such run-on containing a ure undefined entry word.

Hierarchical Context

Top-level member of dictionary entry; when present, occurs just after the top-level def definition section.

Display Guidance

In a set of undefined run-ons, each could be displayed in a new paragraph or kept inline, depending on space considerations.

The ure is displayed in bold, preceded by an em-dash, and followed by a space.

Data Model

"uros" : array of one or more undefined run-on objects, which may contain the members:

"ure" : string  undefined entry word

required fl

"utxt" : array  undefined run-on text section containing vis and/or uns elements

optional ins, lbs, prs, psl, sls, vrs

Example

An undefined run-on in the entry "threatening".

"uros":[
  {
    "ure":"threat*en*ing*ly",
    "prs":[
      {
        "mw":"\u02c8thret-ni\u014b-l\u0113",
        "sound":{"audio":"threat05","ref":"c","stat":"1"}
      },{
        "mw":"\u02c8thre-t\u1d4an-i\u014b-"
      }
    ],
    "fl":"adverb",
    "utxt":[
      [
        "vis",
        [
          {"t":"gestured {wi}threateningly{\/wi}"}
        ]
      ]
    ]
  }
]

    
2.19 Defined Run-Ons: dros
A defined run-on phrase is an expression or phrasal verb formed from the headword. This phrase, its definition section, and other information such as pronunciations, together make up a defined run-on. A set of defined run-ons dros can follow (or "run on" from) the entry's main def definition section, with each such run-on containing a drp defined run-on phrase.

Hierarchical Context

Top-level member of dictionary entry. Occurs after uros if present, otherwise after def.

Display Guidance

Each defined run-on is typically displayed in a new paragraph.

The drp is displayed in bold and preceded by an em-dash.

Data Model

"dros" : array of one or more defined run-on objects, which may contain the members:

"drp" : string  defined run-on phrase

required def

optional et, lbs, prs, psl, sls, vrs

Example

A defined run-on containing a defined run-on phrase drp in the entry "abide".

"dros":[
  {
    "drp":"abide by",
    "def":[
      {
        "sseq":[
          [
            [
              "sense",
              {
                "sn":"1",
                "dt":[
                  [
                    "text",
                    "{bc}to conform to "
                  ],[
                    "vis",
                    [
                      {"t":"{it}abide by{\/it} the rules"}
                    ]
                  ]
                ]
              }
            ]
          ],[
            [
              "sense",
              {
                "sn":"2",
                "dt":[
                  [
                    "text",
                    "{bc}to accept without objection {bc}to
                    {d_link|acquiesce|acquiesce} in "
                  ],[
                    "vis",
                    [
                      {"t":"will {it}abide by{\/it} your decision"}
                    ]
                  ]
                ]
              }
            ]
          ]
        ]
      }
    ]
  }
]
    
    
2.20 Directional Cross-Reference Section: dxnls
Directional cross-references to other entries may be presented after the main definition section. A set of one or more directional cross-references is contained in dxnls.

Hierarchical Context

Top-level member of dictionary entry, occurring near end of entry.

Display Guidance

Display in a separate paragraph.

Data Model

"dxnls" : array of strings containing directional cross-references. See {dxt} for more information.

Example

A dxnls in the geographical entry "Acadia".

"dxnls":[
  "see also {dxt|acadian||}"
]
    
2.21 Usage Section: usages
In addition to usage notes within definitions, a more extensive usage discussion may be included in the entry. A set of usage discussions makes up a usage section, which is contained in usages.

Hierarchical Context

Top-level member of dictionary entry, occurring near end of entry.

Display Guidance

Typically displayed in a separate paragraph, using the pl contents as a heading.

A uarefs is preceded by "usage " in bold, then "see in addition " in normal font, followed by one or more uaref members rendered as hyperlinks to other usage sections.

Data Model

"usages" : array of one or more usage objects containing the members:

"pl" : string  paragraph label: heading to display at top of section

"pt" : [array]  paragraph text containing the elements:

"text", string  usage discussion text

vis

"uarefs" : object  containing one or more:

"uaref" : string  usage see in addition reference: contains the text and ID of a "see in addition" reference to another usage section.

Example

A usages section in the third homograph of "above".

"usages":[
  {
    "pl":"Using {parahw}above{\/parahw} as an Adjective or Noun",
    "pt":[
      [
        "text","Although still objected to by some, the use of {it}above{\/it}
        as a noun to mean \u0022something that is above\u0022 "
      ],[
        "vis",
        [
          {"t":"none of the {it}above{\/it}"},
          {
            "t":"the {it}above{\/it} is Theseus\u0027s opinion",
            "aq":{"auth":"William Blake"}
          }
        ]
      ],[
        "text",
        " and as an adjective "
      ],[
        "vis",
        [
          {
            "t":"without the {it}above{\/it} reserve",
            "aq":{"auth":"O. W. Holmes \u20201935"}
          },{
            "t":"I was brought up on the {it}above{\/it} words",
            "aq":{"auth":"Viscount Montgomery"}
          }
        ]
      ],[
        "text",
        " has been long established as standard."
      ]
    ]
  }
]
    
    
2.22 Synonyms Section: syns
Extensive discussions of synonyms for the headword may be included in the entry. A set of such synonym discussions makes up a synonym section, which is contained in syns.

Hierarchical Context

Top-level member of dictionary entry, occurring near end of entry.

Display Guidance

Typically displayed in a separate paragraph with a heading such as "Synonym Discussion of [headword]".

An sarefs is preceded by "synonyms " in bold, then "see in addition " in normal font, then the sarefs array elements rendered as hyperlinks to other synonym sections.

Data Model

"syns" : array of one or more synonym discussion objects containing the members:

"pl" : string  paragraph label: heading to display at top of section

"pt" : [array]  paragraph text containing the elements:

"text", string  synonym discussion text

vis

"sarefs" : array  see in addition reference: contains one or more elements, each of which is the text and ID of a "see in addition" reference to another synonym section.

Example

A syns synonym section in the entry "agree".

"syns":[
  {
    "pl":"synonyms",
    "pt":[
      [
        "text",
        "{sc}agree{\/sc} {sc}concur{\/sc} {sc}coincide{\/sc} mean to come
        into or be in harmony regarding a matter of opinion. {sc}agree{\/sc}
        implies complete accord usually attained by discussion and
        adjustment of differences. "
      ],[
        "vis",
        [
          {"t":"on some points we all can {it}agree{\/it}"}
        ]
      ],[
        "text",
        " {sc}concur{\/sc} often implies approval of someone else\u0027s
        statement or decision. "
      ],[
        "vis",
        [
          {"t":"if my wife {it}concurs{\/it}, it\u0027s a deal"}
        ]
      ],[
        "text",
        " {sc}coincide{\/sc}, used more often of opinions, judgments,
        wishes, or interests than of people, implies total agreement. "
      ],[
        "vis",
        [
          {"t":"their wishes {it}coincide{\/it} exactly with my desire"}
        ]
      ]
    ],
    "sarefs":[
      "assent"
    ]
  }
]

    
2.23 Quotations Section: quotes
In addition to the verbal illustrations provided throughout the entry, a larger section of quotations from cited sources is sometimes included. A set of quotes is contained in quotes.

Hierarchical Context

Top-level member of dictionary entry, occurring near end of entry.

Display Guidance

Quotations are typically displayed in a separate paragraph with a heading such as "Examples of [headword] in a Sentence".

Data Model

"quotes" : array of one or more quote objects containing the members:

"t" : string  quotation text (required)

aq (required)

Example

A quotes section at the entry for "tedious".

quotes":[
  {
    "t":"Another of their assignments was to slow-fly any plane that had
    a new engine to break it in; that meant flying the aircraft for a
    {qword}tedious{\/qword} hour-and-a-half as slowly as it would possibly
    go without falling out of the sky.",
    "aq":{
      "auth":"Doris Weatherford",
      "source":"{it}American Women and World War II{\/it}",
      "aqdate":"1990"
    }
  },{
    "t":"Writing a new spreadsheet or word-processing program these
    days is a {qword}tedious{\/qword} process, like building a skyscraper
    out of toothpicks.",
    "aq":{
      "auth":"Jeff Goodell",
      "source":"{it}Rolling Stone{\/it}",
      "aqdate":"16 June 1994"
    }
  },{
    "t":"From there, it became clear that the deposition was going to
    be neither as undramatic nor as quotidian, and even {qword}tedious{\/qword},
    as it at first appeared.",
    "aq":{
      "auth":"Renata Adler",
      "source":"{it}New Yorker{\/it}",
      "aqdate":"June 23, 1986"
    }
  }
],


    
2.24 Artwork: art
Entries may have illustrations to provide a visual depiction of the headword. All information needed to display an image is contained in art.

Hierarchical Context

Top-level member of dictionary entry, occurring near the end of the entry.

Display Guidance

When displaying an image on a page, it is typically preceded by lead-in text such as "Illustration of [headword]".

See Data Model below for more information on linking to a Merriam-Webster page containing both image and caption vs linking directly to the image.

Data Model

"art" : object  containing the following members:

"artid" : string  filename of target image

"capt" : string  image caption text

1. If you want to link to a separate page containing both image and caption, the URL should be in the following form: https://www.merriam-webster.com/art/dict/[base filename].htm where [base filename] equals the value of artid. For the Example below, this URL would be: https://www.merriam-webster.com/art/dict/heart.htm

2. If you prefer to link directly to the image, the URL should be in the following form: https://www.merriam-webster.com/assets/mw/static/art/dict/[base filename].gif where [base filename] equals the value of artid. Use the content of capt to pull in the caption content. For the Example below, this URL would be: https://www.merriam-webster.com/assets/mw/static/art/dict/heart.gif

Example

An art object in the entry for "heart".

"art":{
  "artid":"heart",
  "capt":"heart 1a: {it}1{\/it} aorta, {it}2{\/it} pulmonary artery,
  {it}3{\/it} left atrium, {it}4{\/it} left ventricle, {it}5{\/it} right ventricle,
  {it}6{\/it} right atrium"
}
    
    
2.25 Tables: table
A reference from an entry to a table is contained in table.

Hierarchical Context

Top-level member of dictionary entry, occurring near the end of the entry.

Display Guidance

Typically presented as a link in a separate paragraph, where the link text is provided by displayname. The table is generally presented as an image on a separate page.

Data Model

"table" : object  containing the following members:

"tableid" : string  ID of the target table

"displayname" : string  text to display in table link

A table reference URL should be in the following form: https://www.merriam-webster.com/table/collegiate/[base filename].htm where [base filename] equals the value of tableid.

Example

A table reference in the Collegiate entry for "alphabet". If you were to generate a table reference URL here, it would be: https://www.merriam-webster.com/table/collegiate/alphabet.htm

"table":{
  "tableid":"alphabet",
  "displayname":"Alphabet Table"
}
    

    
2.26 Etymology: et
An etymology is an explanation of the historical origin of a word. While the etymology contained in an et most typically relates to the headword, it may also explain the origin of a defined run-on phrase or a particular sense.

Hierarchical Context

Occurs as top-level member of dictionary entry and in dros, sdsense, sen, or sense.

Display Guidance

Typically displayed inline within square brackets or in its own block with a heading such as "Word Origin".

Data Model

"et" : array consisting of one or more elements:

["text", string]	contains the etymology content (required)
["et_snote",[["t", string]]]	contains a supplemental information note for the etymology (optional)
Example

The etymology for the first homograph of "traffic".

"et":[
  [
    "text",
    "Middle French {it}trafique{\/it}, from Old Italian {it}traffico{\/it},
     from {it}trafficare{\/it} to trade in coastal waters"
  ]
]
    
    
2.27 First Known Use: date
The date of the earliest recorded use of a headword in English is captured in date.

Hierarchical Context

Top-level member of dictionary entry

Display Guidance

Typically displayed inline within parentheses or in its own block with a heading such as "First Known Use".

Data Model

"date" : string

Example

"hom":4,
"hwi":{"hw":"test"},
[...]
"date":"circa 1842"
    
    
2.28 Short Definition: shortdef
A short definition provides a highly abridged version of the main definition section, consisting of just the definition text for the first three senses. It is not meant to be displayed along with the full entry, but instead as an alternative, shortened preview of the entry content. A short definition is contained in a shortdef.

Hierarchical Context

Top-level member of the dictionary entry, occurring at the very end of the entry.

Display Guidance

Each element in the shortdef array represents a distinct definition, and needs to be separated from the next element by a newline, sense number, or the like.

Note this section should not be displayed alongside the main definition section content, but only in specialized contexts where a preview or shortened entry view is needed.

Data Model

"shortdef" : array of one or more elements

Example

A short definition in the entry "abide".

"shortdef":[
  "to bear patiently : tolerate",
  "to endure without yielding : withstand",
  "to wait for : await"
]
    
XML Equivalent

There is no XML equivalent for shortdef.

2.29 Tokens Used in Running Text
2.29.1 Formatting and Punctuation Tokens: {b}, {bc}, {inf}, {it}, {ldquo}, {p_br), {rdquo}, {sc}, {sup}
Within running text, we sometimes mark up text according to its intended presentation (eg, in bold or italic font) rather than its underlying semantics. Typographic tokens are used for this purpose: {b} for bold, {it} for italics, {sc} for smallcaps, {inf} for subscript, and {sup} for superscript. The token {p_br} indicates where a paragraph break should be inserted.

Punctuation marks may also be represented as tokens: {ldquo} for left double quotes, {rdquo} for right double quotes, and {bc} for the ubiquitous bold colon.

Hierarchical Context

May occur in many string contexts, but most typically found in dt, t, and text.

Display Guidance / Data Model

{b}string{\/b}  display text in bold

{bc}  output a bold colon and a space

{inf}string{\/inf}  display text in subscript

{it}string{\/it}  display text in italics

{ldquo}  output a left double quote character (U+201C)

{rdquo}  output a right double quote character (U+201D)

{sc}string{\/sc}  display text in small capitals

{sup}string{\/sup}  display text in superscript

Example

Use of {bc}, {it}, {inf} and {sup} tokens in the entry "ammonium".

"def":[
  {
    "sseq":[
      [
        [
          "sense",
          {
            "dt":[
              [
                "text",
                "{bc}an ion NH{inf}4{\/inf}{sup}+{\/sup} derived from
                {a_link|ammonia} by combination with a hydrogen ion and
                known in compounds (such as salts) that resemble in properties
                the compounds of the alkali metals"
              ]
            ]
          }
        ]
      ]
    ]
  }
],
"et":[
  [
    "text",
    "New Latin, from {it}ammonia{\/it}"
  ]
],
"date":"1808"
    
    
2.29.2 Word-Marking and Gloss Tokens: {gloss}, {parahw}, {phrase}, {qword}, {wi}
Within definitions, quotations, paragraph labels, and verbal illustrations, we note instances of the word or phrase being defined or exemplified. A single such word in text is marked with a {wi} token, while a phrase is marked with a {phrase} token. In quotations, a use of the word is marked with a qword token. Finally, the headword in a paragraph label is marked by a {parahw}.

At times the use of a word or phrase in a verbal illustration or quote may require an editorial comment (= gloss) on its meaning in that particular context, which is contained in a gloss token.

Hierarchical Context

May occur in many string contexts, but most typically found in dt, pl, t, and text.

Display Guidance

A phrase is typically displayed in bold italics, while qword and wi are typically displayed in italics.

A parahw is typically displayed in bold smallcaps.

A gloss is displayed within square brackets and in normal font.

Data Model

{gloss}string{\/gloss}  encloses a gloss explaining how a word or phrase is used in a particular context

{parahw}string{\/qword}  encloses an instance of the headword within a paragraph label

{phrase}string{\/phrase}  encloses a phrase in running text (this may be a phrase containing the headword or a defined run-on phrase)

{qword}string{\/qword}  encloses an instance of the headword within a quote

{wi}string{\/wi}  encloses an instance of the headword used in running text

Example

Use of {gloss}, {phrase}, {qword}, and {wi} tokens in the entry "absence".

[
  "sense",
  {
    "sn":"1",
    "dt":[
      [
        "text",
        "{bc}a state or condition in which something expected, wanted,
        or looked for is not present or does not exist {bc}a state or
        condition in which something is {d_link|absent|absent:1} "
      ],[
        "vis",
        [
          {"t":"an {wi}absence{\/wi} {gloss}=lack{\/gloss} of detail"},
          {"t":"{phrase}In the absence of{\/phrase} reform
          {gloss}=without reform{\/gloss}, progress will be slow."}
        ]
      ]
    ]
  }
]
[...]
"quotes":[
  {
    "t":"Only five to six inches long and weighing less than two ounces,
    the elf owl is the smallest bird of prey in the world. Its bobbed
    tail, white \u0022eyebrows,\u0022 and {qword}absence{\/qword} of ear
    tufts give this tiny mothlike predator its impish appearance.",
    "aq":{
      "auth":"Ken Lamerton",
      "source":"{it}Bird Watcher\u0027s Digest{\/it}",
      "aqdate":"May\/June 1996"
    }
  }
  [...]
]
    
    
2.29.3 Cross-Reference Grouping Tokens: {dx}, {dx_def}, {dx_ety}, {ma}
Groups together introductory text and one or more cross-references. Cross-reference grouping tokens include {dx}, {dx_def}, {dx_ety}, and {ma}.

See Cross-Reference Tokens for detailed information on the cross-references themselves.

Hierarchical Context

{dx} and {dx_def} occur in many string contexts, but are most typically found in dt, t, and text.

{dx_ety} and {ma} occur in et.

Display Guidance

Display an em-dash before {dx}, {dx_ety}, and {ma}. At {ma}, display the text "more at " after the em-dash.

Display {dx_def} within parentheses.

Data Model

{dx}string{\/dx}  encloses introductory text and one or more {dxt} cross-reference tokens

{dx_def}string{\/dx_def}  used for a parenthetical cross-reference; encloses introductory text and one or more {dxt} cross-reference tokens

{dx_ety}string{\/dx_ety}  used for a directional cross-reference within an etymology; encloses introductory text and one or more {dxt} cross-reference tokens

{ma}string{\/ma}  used for a "more at" informational cross-reference within an etymology; encloses introductory text and one or more {mat} tokens

Example

Uses of {dx}, {dx_def} and {ma} in the entry "fowl".

def":[
  {
    "sseq":[
      [
        [
          "sense",
          {
            "sn":"1",
            "dt":[
              [
                "text",
                "{bc}a bird of any kind {dx}compare {dxt|waterfowl||},
                {dxt|wildfowl||}{\/dx}"
              ]
            ]
          }
        ]
      ],[
        [
          "sense",
          {
            "sn":"2 a",
            "dt":[
              [
                "text",
                "{bc}a cock or hen of the domestic chicken ({it}Gallus
                gallus{\/it})"
              ]
            ],
            "sdsense":{
              "sd":"especially",
              "dt":[
                [
                  "text",
                  "{bc}an adult hen"
                ]
              ]
            }
          }
        ],[
          "sense",
          {
            "sn":"b",
            "dt":[
              [
                "text",
                "{bc}any of several domesticated {dx_def}see {dxt|domesticate:1||2}
                {\/dx_def} or wild {d_link|gallinaceous|gallinaceous}
                birds {dx}compare {dxt|guinea fowl||}, {dxt|jungle fowl||}{\/dx}"
              ]
            ]
          }
        ]
      ],[
        [
          "sense",
          {
            "sn":"3",
            "dt":[
              [
                "text",
                "{bc}the meat of {a_link|fowls} used as food "
              ],[
                "vis",
                [
                  {
                    "t":"roasted {wi}fowl{\/wi}"
                  }
                ]
              ]
            ]
          }
        ]
      ]
    ]
  }
],
"et":[
  [
    "text",
    "Middle English {it}foul{\/it}, from Old English {it}fugel{\/it};
    akin to Old High German {it}fogal{\/it} bird, and probably to Old
    English {it}fl\u0113ogan{\/it} to fly
    {ma}{mat|fly|}{\/ma}"
  ]
]
    
    
2.29.4 Cross-Reference Tokens: {a_link}, {d_link}, {dxt}, {et_link}, {i_link}, {mat}, {sx}
A cross-reference may refer to an entry, a sense within an entry, an illustration, or a table. Cross-references are always self-contained tokens with information on the link text, link target, and any additional text to display. They are contained in {a_link}, {d_link}, {dxt}, {et_link}, {i_link}, {mat}, and {sx}.

Hierarchical Context

May occur in many string contexts, but most typically found in dt, et, t, and text.

{dxt} always occurs within a {dx}, {dx_def}, or {dx_ety} token.

{mat} always occurs within a {ma} token.

Display Guidance

Generate a hyperlink, obtaining link text and target ID as described in Data Model below. Note that the {dxt} and {sx} tokens may contain additional text to be displayed after the link text.

Link text is typically displayed in smallcaps, except for {a_link} and {d_link} which are displayed in normal font, and {i_link} which is displayed in italics.

Data Model

The tokens contain fields separated by a pipe character ("|"). There are at least 2 fields and a maximum of 4 fields within these tokens, indicated in the "Token Format" column. If a field does not exist for a particular token, it will be marked "n/a".


Token Format	Field 2	Field 3	Field 4
{a_link|} (auto link)	hyperlink text and link target (note this may not correspond exactly to an id)	n/a	n/a
{d_link||} (direct link)	hyperlink text	if non-empty, use as target id; otherwise use field 2	n/a
{i_link||} (italicized link)	hyperlink text	if non-empty, use as target id; otherwise use field 2	n/a
{et_link||} (etymology link)	hyperlink text	if non-empty, use as target id; otherwise use field 2	n/a
{mat||} (more at target)	hyperlink text	if non-empty, use as target id; otherwise use field 2	n/a
{sx|||} (synonymous cross-reference)	hyperlink text	if non-empty, use as target id; otherwise use field 2	sense number and transitive/intransitive information (if any) to display after link text
{dxt|||} (directional cross-reference target)	hyperlink text	
if field 4 contains "illustration", field 3 is the target artid—form URL as described under art;
if field 4 contains "table", field 3 is the target tableid—form URL as described under table;
otherwise if non-empty, use as target id;
otherwise use field 2 as target id
sense number and transitive/intransitive information (if any) or text "illustration" or "table".
This should be displayed after link text except for "table", which should not be displayed.
Example

Use of {a_link}, {d_link}, {dxt}, {mat} and {sx} in the first homograph of "monitor".

"def":[
  {
    "sseq":[
      [
        [
          "sense",
          {
            "sn":"1 a",
            "dt":[
              [
                "text",
                "{bc}a student appointed to assist a teacher"
              ]
            ]
          }
        ],[
          "sense",
          {
            "sn":"b",
            "dt":[
              [
                "text",
                "{bc}one that warns or instructs "
              ],[
                "vis",
                [
                  {
                    "t":"{wi}monitors{\/wi} and instructors for troops
                    green in the art of war",
                    "aq":{
                      "source":"{it}New York Times{\/it}"
                    }
                  }
                ]
              ]
            ]
          }
        ],[
          "pseq",
          [
            [
              "bs",
              {
                "sense":{
                  "sn":"c",
                  "dt":[
                    [
                      "text",
                      "{bc}one that {a_link|monitors} or is used in
                      {a_link|monitoring}: such as"
                    ]
                  ]
                }
              }
            ],[
              "sense",
              {
                "sn":"(1)",
                "dt":[
                  [
                    "text",
                    "{bc}an electronic device with a screen used for
                    display (as of television pictures or computer
                    information)"
                  ]
                ]
              }
            ],[
              "sense",
              {
                "sn":"(2)",
                "dt":[
                  [
                    "text",
                    "{bc}a device for observing a biological condition
                    or function "
                  ],[
                    "vis",
                    [
                      {"t":"a heart {wi}monitor{\/wi}"}
                    ]
                  ]
                ]
              }
            ]
          ]
        ]
      ],[
        [
          "sense",
          {
            "sn":"2",
            "dt":[
              [
                "text",
                "{bc}{sx|monitor lizard||}"
              ]
            ]
          }
        ]
      ],[
        [
          "sen",
          {
            "sn":"3",
            "et":[
              [
                "text",
                "{it}Monitor{\/it}, first ship of the type"
              ]
            ]
          }
        ],[
          "sense",
          {
            "sn":"a",
            "dt":[
              [
                "text",
                "{bc}a heavily armored warship formerly used in coastal
                operations having a very low {d_link|freeboard|freeboard}
                and one or more revolving gun turrets"
              ]
            ]
          }
        ],[
          "sense",
          {
            "sn":"b",
            "dt":[
              [
                "text",
                "{bc}a small modern warship with shallow draft
                {dx_def}see {dxt|draft:1||8}{\/dx_def} for coastal bombardment"
              ]
            ]
          }
        ]
      ],[
        [
          "sense",
          {
            "sn":"4",
            "dt":[
              [
                "text",
                "{bc}a raised central portion of a roof having low
                windows or louvers for providing light and air"
              ]
            ]
          }
        ]
      ]
    ]
  }
],
[...]
"et":[
  [
    "text",
    "Latin, one that warns, overseer, from {it}mon\u0113re{\/it} to
    warn {ma}{mat|mind|}{\/ma}"
  ]
]
    
    
2.29.5 Date Sense Token: {ds}
A date sense token links information on the first known use of a headword to a particular sense within the entry. The date sense token {ds} only occurs within date.

Hierarchical Context

Occurs in date.

Display Guidance

Typically displayed in italics and preceded by the heading "in the meaning defined at ".

Nothing should be output if fields 2 through 5 are empty (eg, {ds||||}).

Data Model

This token contains 5 fields separated by a pipe character ("|").

Token Format	Field 2	Field 3	Field 4	Field 5
{ds||||}	at entries with verb dividers, contains "t" for transitive and "i" for intransitive verbs; otherwise empty.	bold sense number, if any	lowercase letter sense number, if any	parenthesized sense number, if any
Example

A {ds} in the entry "reap", indicating that the date information relates to transitive sense number 1a(1).

"date":"before 12th century{ds|t|1|a|1}"
