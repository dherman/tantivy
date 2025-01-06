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

function cover(matches, length) {
  let result = [];
  let nextOffset = 0;
  for (let i = 0; i < matches.length; i++) {
    let match = matches[i];
    if (nextOffset < match.charOffsetFrom) {
      result.push({
        match: false,
        charOffsetFrom: nextOffset,
        charOffsetTo: match.charOffsetFrom,
      });
    }
    result.push({
      match: true,
      charOffsetFrom: match.charOffsetFrom,
      charOffsetTo: match.charOffsetTo,
    });
    nextOffset = match.charOffsetTo;
  }
  if (nextOffset < length) {
    result.push({
      match: false,
      charOffsetFrom: nextOffset,
      charOffsetTo: length,
    });
  }
  return result;
}

function withEmphasis(text) {
  const split = text.split(/(_\w+_)/);
  return <>{split.map((frag, i) => (frag.startsWith('_') && frag.endsWith('_')) ? <em key={i}>{frag.substring(1, frag.length - 1)}</em> : frag)}</>
}

function highlight(clip, matches) {
  const ranges = cover(matches, clip.length);
  return ranges.map(range => {
    return range.match
      ? <span key={range.charOffsetFrom} className = "search-highlight">{withEmphasis(clip.substring(range.charOffsetFrom, range.charOffsetTo))}</span>
      : <span key={range.charOffsetFrom}>{withEmphasis(clip.substring(range.charOffsetFrom, range.charOffsetTo))}</span>;
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
        setOptions(items.map(item => item.join(' ')));
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
          {selectedPhrase.length > 0 && searchResults.length === 0 && (
            <div className="text-center">
              <img className="no-results" src="jane-wut.png" alt="No Results Found"/>
            </div>
          )}
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
                    <>{highlight(result.text, result.matches /*selectedPhrase[0]*/)}</>
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
