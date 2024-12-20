import { useState } from 'react';
import { AsyncTypeahead } from 'react-bootstrap-typeahead';
import 'bootstrap/dist/css/bootstrap.min.css';
import './App.css';
import 'react-bootstrap-typeahead/css/Typeahead.css';

const SEARCH_URI = 'http://localhost:5174/';

function volumeName(volume) {
  return volume ? `Vol. ${volume}, ` : '';
}

function chapterName(chapter) {
  const n = parseInt(chapter);
  return isNaN(n) ? chapter : `Ch. ${n}`;
}

function highlight(clip, query) {
  const words = query.split(/\s+/);
  const regex = new RegExp('(' + words.join('|') + ')', 'gi');
  const fragments = clip.split(regex);
  return clip.split(regex).map(fragment => {
    return words.includes(fragment) ? <strong>{fragment}</strong> : fragment;
  });
}

function App() {
  const [isLoading, setIsLoading] = useState(false);
  const [selectedClip, setSelectedClip] = useState([]);
  const [options, setOptions] = useState([]);

  const handleSearch = (query) => {
    setIsLoading(true);
        
    console.error(`handleSearch: "${query}"`);

    fetch(`${SEARCH_URI}?q=${encodeURI(query)}`)
      .then((resp) => resp.json())
      .then(({ items }) => {
        console.error(items);
        setOptions(items);
        setIsLoading(false);
      })
      .catch(err => { console.error(err); });
  };

  const filterBy = () => true;

  return (
    <>
      <div className="container">
        <div className="search-container">
          <h1 className="text-center mb-4">Search the Jane Austen library</h1>
          <AsyncTypeahead
            filterBy={filterBy}
            id="github-username-search"
            isLoading={isLoading}   
            labelKey="query"
            minLength={3}
            onSearch={handleSearch}
            onChange={setSelectedClip}
            options={options}
            placeholder="Enter a word or phrase to search for"
            selected={selectedClip}
            renderMenuItemChildren={(option) => (
              <>
                <img
                  alt={option.title}
                  src={option.icon}
                  style={{
                    height: '75px',
                    marginRight: '10px',
                    width: '50px',
                  }}
                />
                <span><>{highlight(option.text, option.query)}</></span>
              </>
            )}
          />
          {selectedClip.length > 0 && (
            <div className="mt-3 text-center">
              <blockquote>{selectedClip[0].text}</blockquote>
              <p><em>{selectedClip[0].title}</em>, {volumeName(selectedClip[0].volume)}{chapterName(selectedClip[0].chapter)}</p>
            </div>
          )}
        </div>
      </div>
    </>
  )
}

export default App;
