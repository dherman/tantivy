import { useState } from 'react';
import { AsyncTypeahead } from 'react-bootstrap-typeahead';
import 'bootstrap/dist/css/bootstrap.min.css';
import './App.css';
import 'react-bootstrap-typeahead/css/Typeahead.css';

const SEARCH_URI = 'http://localhost:5174/search/';
const TYPEAHEAD_URI = 'http://localhost:5174/typeahead/';

function volumeName(volume) {
  return volume ? `Vol. ${volume}, ` : '';
}

function chapterName(chapter) {
  const n = parseInt(chapter);
  return isNaN(n) ? chapter : `Ch. ${n}`;
}

// function highlight(clip, query) {
//   const words = query.split(/\s+/);
//   const regex = new RegExp('(' + words.join('|') + ')', 'gi');
//   const fragments = clip.split(regex);
//   return clip.split(regex).map(fragment => {
//     return words.includes(fragment) ? <strong>{fragment}</strong> : fragment;
//   });
// }

function highlightFragments(frag, clipIndex, query) {
  let innerClipIndex = 0;
  return frag.split(new RegExp("(" + query + ")", "i")).map(frag => {
    const key = clipIndex + innerClipIndex;
    const result = (frag.toLowerCase() === query.toLowerCase())
      ? <span className="search-highlight" key={key}>{frag}</span>
      : <span key={key}>{frag}</span>;
    innerClipIndex += frag.length;
    return result;
  });
}

function highlight(clip, query) {
  let clipIndex = 0;
  return clip.split(/(_[^_]+_)/).flatMap(frag => {
    const result = (frag.startsWith('_') && frag.endsWith('_'))
      ? (<em key={clipIndex}>{frag.substring(1, frag.length - 1)}</em>)
      : (highlightFragments(frag, clipIndex, query));
    clipIndex += frag.length;
    return result;
    // if (frag.startsWith('_') && frag.endsWith('_')) {
    //   const copy = frag.substring(1, frag.length - 1);
    //   clipIndex += frag.length;
    //   return <em>{copy}</em>;
    // } else {
    //   return <>{highlightFragments(frag, clipIndex, query)}</>;
    // }
  });
}
function App() {
  const [isLoading, setIsLoading] = useState(false);
  const [selectedPhrase, setSelectedPhrase] = useState([]);
  const [options, setOptions] = useState([]);
  const [searchResults, setSearchResults] = useState([]);
  const [typeaheadTime, setTypeaheadTime] = useState(null);
  const [searchTime, setSearchTime] = useState(null);

  const handleSelected = (selected) => {
    setSearchTime(null);
    setSelectedPhrase(selected);
    fetch(`${SEARCH_URI}?q=${encodeURI(selected[0])}`)
      .then((resp) => resp.json())
      .then(({ time, items }) => {
        setSearchTime(time);
        setSearchResults(items);
      })
      .catch(err => { console.error(err); });
  }

  const handleTypeahead = (query) => {
    setIsLoading(true);
    setSearchResults([]);
    setTypeaheadTime(null);

    fetch(`${TYPEAHEAD_URI}?q=${encodeURI(query)}`)
      .then((resp) => resp.json())
      .then(({ time, items }) => {
        setTypeaheadTime(time);
        setOptions(items);
        setIsLoading(false);
      })
      .catch(err => { console.error(err); });
  };

  const filterBy = () => true;

  return (
    <>
      <div className="container">
        <div className="timings">
          <table>
            <tbody>
              <tr>
                <td className="label">Typeahead</td>
                <td className="value">
                  {typeaheadTime !== null ? `${Math.round(typeaheadTime * 10) / 10}ms` : '–'}
                </td>
              </tr>
              <tr>
                <td className="label">Search</td>
                <td className="value">
                  {searchTime !== null ? `${Math.round(searchTime * 10) / 10}ms` : '–'}
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <div className="search-container">
          <h1 className="text-center mb-4">Search the Jane Austen library</h1>
          <AsyncTypeahead
            filterBy={filterBy}
            id="jane-austen-typeahead"
            isLoading={isLoading}   
            minLength={1}
            onSearch={handleTypeahead}
            onChange={handleSelected}
            options={options}
            placeholder="Enter a word or phrase to search for"
            selected={selectedPhrase}
            renderMenuItemChildren={option => (<span>{option}</span>)}
          />
          {searchResults.length > 0 && selectedPhrase.length > 0 && (
            <div className="results-container">
            {searchResults.map((result, i) => {
              return (
                <div className="mt-3 text-center" key={i}>
                  <blockquote>
                    <a href={result.url}>
                      <img
                        src={result.icon}
                        alt={result.title}
                        className="search-thumbnail"
                      />
                    </a>
                    <>{highlight(result.text, selectedPhrase[0])}</>
                  </blockquote>
                  <p className="search-result-attribution"><a href={result.url}><em>{result.title}</em></a>, {volumeName(result.volume)}{chapterName(result.chapter)}</p>
                </div>
              );
            })}
            </div>
          )}
        </div>
      </div>
    </>
  )
}

export default App;
